use tokio::prelude::*;
use std::task::{Poll, Context};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use std::io::{Error as IoError, ErrorKind as IoErrorKind};
use crate::config::{Tunnel};
use std::fmt::Debug;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use futures::{ready, Future};


#[derive(Clone)]
pub struct ProxyConnector;

impl ProxyConnector {

    pub async fn connect(addr: SocketAddr, 
        tunnel: Tunnel, 
        proxy: Option<SocketAddr>, 
        user: Option<String>) -> io::Result<TcpStream> {
        let (remote_socket, tunnel_to) = match proxy {
            None => {
                debug!(
                    "Connecting directly to {}",
                    addr
                );
              (TcpStream::connect(&addr).await?, None)
            }
            Some(p) => {
                debug!("Connecting via proxy {}", p);
                match TcpStream::connect(p).await {
                    Ok(s) => (s,Some(tunnel)),
                    Err(e) => {
                        warn!("Proxy connection failed {:?}, trying direct", e);
                        (TcpStream::connect(&addr).await?, None)
                    }  
                    
                    
                }
                }
            };
            
        let half_connected = Self::write_proxy_connect(remote_socket, tunnel_to.clone(), user).await?;
        Self::read_proxy_response(half_connected, tunnel_to.is_some()).await
    }

    async fn write_proxy_connect(mut stream: TcpStream, tun: Option<Tunnel>, user: Option<String>) -> io::Result<TcpStream> {
        let connect_string = match tun {
        
        Some(tun) => {
            let mut s =format!(
                "CONNECT {}:{} HTTP/1.1\r\n",
                &tun.remote_host,
                tun.remote_port
                );
            if let Some(u) = user {
                s.push_str(&format!("Proxy-Authorization: Basic {}\r\n", u));
            };
            s.push_str("\r\n");
            s
        } 
        
        None => {
            "".to_owned()
        }
        };
        stream.write_all(connect_string.as_bytes()).await?;
        Ok(stream)
    }

    fn read_proxy_response(s: TcpStream, is_proxied: bool) -> ConnectResponse {
        ConnectResponse {
            stream: Some(s),
            status: Status::Started,
            is_proxied
        }
    }

}


#[derive(PartialEq, Debug, Clone, Copy)]
enum Status {
    Started,
    HeaderOk,
    FirstCr,
    FirstLf,
    SecondCr,
    Done,
}

struct ConnectResponse {
    stream: Option<TcpStream>,
    is_proxied: bool,
    status: Status,
}

macro_rules! pin_poll {
    ($f:expr, $cx:ident) => {
        let r = &mut $f;
        let f = Pin::new(r);
        ready!(Future::poll(f,$cx));
    };
}

impl Future for ConnectResponse {
    type Output = io::Result<TcpStream>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let Self {is_proxied, stream, status} = &mut *self;
        if *is_proxied {
            let s = stream.as_mut().unwrap();

            if *status == Status::Started {
                let mut header = [0; 12];
                pin_poll!(s.read_exact(&mut header), cx);
                
                // check status code of proxy response
                let result_code = match ::std::str::from_utf8(&header) {
                    Err(_) => return Poll::Ready(Err(other_error("Invalid header - not UTF8"))),
                    Ok(s) => match str::parse::<u16>(&s[9..12]) {
                        Ok(n) => n,
                        Err(_) => return Poll::Ready(Err(other_error("Invalid status - not number"))),
                    },
                };

                if result_code < 200 || result_code >= 300 {
                    return Poll::Ready(Err(other_error(&format!("Invalid status - {}",result_code))));
                }

                *status = Status::HeaderOk
            }

            loop {
                let mut next_byte = [0; 1];
                pin_poll!(s.read_exact(&mut next_byte), cx);

                match (*status, next_byte[0]) {
                    (Status::HeaderOk, b'\r') => *status = Status::FirstCr,
                    (Status::FirstCr, b'\n') => *status = Status::FirstLf,
                    (Status::FirstCr, _) => return Poll::Ready(Err(other_error("Invalid end of line"))),
                    (Status::FirstLf, b'\r') => *status = Status::SecondCr,
                    (Status::FirstLf, _) => *status = Status::HeaderOk,
                    (Status::SecondCr, b'\n') => break,
                    (Status::SecondCr, _) => return Poll::Ready(Err(other_error("Invalid end of line"))),
                    (Status::HeaderOk, _) => (),
                    (_, _) => {
                        return Poll::Ready(Err(other_error(&format!(
                            "Invalid Header - status {:?} on byte: {:?}",
                            &self.status,
                            next_byte[0]
                        ))))
                    }
                }
            }
        }
        self.status = Status::Done;
        Poll::Ready(Ok(self.stream.take().unwrap().into()))
    }
}

fn other_error(text: &str) -> IoError {
    IoError::new(IoErrorKind::Other, text)
}

/*
#[derive(Clone)]
pub struct FixedTcpStream(Arc<TcpStream>);

impl From<TcpStream> for FixedTcpStream {
    fn from(s: TcpStream) -> Self {
        FixedTcpStream(Arc::new(s))
    }
}

impl io::Read for FixedTcpStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        (&*self.0).read(buf)
    }
}

impl AsyncRead for FixedTcpStream {}

impl io::Write for FixedTcpStream {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        (&*self.0).write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        (&*self.0).flush()
    }
}

impl AsyncWrite for FixedTcpStream {
    fn shutdown(&mut self) -> Poll<(), IoError> {
        self.0.shutdown(Shutdown::Write)?;
        Ok(().into())
    }
}


*/

#[cfg(test)]
mod tests {

    
}
