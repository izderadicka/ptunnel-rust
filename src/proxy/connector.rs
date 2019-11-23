use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::config::{Tunnel};
use std::fmt::Debug;
use std::io;
use std::net::SocketAddr;


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

    async fn read_proxy_response(mut s: TcpStream, is_proxied: bool) -> io::Result<TcpStream> {
        
        if is_proxied {
                let mut header = [0; 12];
                s.read_exact(&mut header).await?;
                
                // check status code of proxy response
                let result_code = match ::std::str::from_utf8(&header) {
                    Err(_) => return Err(other_error("Invalid header - not UTF8")),
                    Ok(s) => match str::parse::<u16>(&s[9..12]) {
                        Ok(n) => n,
                        Err(_) => return Err(other_error("Invalid status - not number")),
                    },
                };

                if result_code < 200 || result_code >= 300 {
                    return Err(other_error(&format!("Invalid status - {}",result_code)));
                }

            let mut status = Status::HeaderOk;
            

            loop {
                let mut next_byte = [0; 1];
                s.read_exact(&mut next_byte).await?;

                match (status, next_byte[0]) {
                    (Status::HeaderOk, b'\r') => status = Status::FirstCr,
                    (Status::FirstCr, b'\n') => status = Status::FirstLf,
                    (Status::FirstCr, _) => return Err(other_error("Invalid end of line")),
                    (Status::FirstLf, b'\r') => status = Status::SecondCr,
                    (Status::FirstLf, _) => status = Status::HeaderOk,
                    (Status::SecondCr, b'\n') => break,
                    (Status::SecondCr, _) => return Err(other_error("Invalid end of line")),
                    (Status::HeaderOk, _) => (),
                }
            }
        }
        Ok(s)
    }

}


#[derive(PartialEq, Debug, Clone, Copy)]
enum Status {
    HeaderOk,
    FirstCr,
    FirstLf,
    SecondCr,
}



fn other_error(text: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, text)
}

#[cfg(test)]
mod tests {

    
}
