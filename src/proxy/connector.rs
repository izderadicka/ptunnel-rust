use crate::config::{Proxy, Tunnel};
use std::fmt::Debug;
use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Clone)]
pub struct ProxyConnector;

impl ProxyConnector {
    pub async fn connect(
        tunnel: Tunnel,
        proxy: Option<Proxy>,
        user: Option<String>,
    ) -> io::Result<TcpStream> {
        let (mut remote_socket, tunnel_through) = match proxy {
            None => {
                debug!("Connecting directly to {:?}", tunnel.remote_addr());
                (TcpStream::connect(tunnel.remote_addr()).await?, None)
            }
            Some(p) => {
                debug!("Connecting via proxy {:?}", p);
                match TcpStream::connect(p.addr()).await {
                    Ok(s) => (s, Some(tunnel)),
                    Err(e) => {
                        warn!("Proxy connection failed {:?}, trying direct", e);
                        (TcpStream::connect(tunnel.remote_addr()).await?, None)
                    }
                }
            }
        };
        if let Some(tunnel) = tunnel_through {
            Self::write_proxy_connect(&mut remote_socket, tunnel, user).await?;
            Self::read_proxy_response(&mut remote_socket).await?
        }

        Ok(remote_socket)
    }

    async fn write_proxy_connect(
        stream: &mut TcpStream,
        tunnel: Tunnel,
        user: Option<String>,
    ) -> io::Result<()> {
        let mut s = format!(
            "CONNECT {}:{} HTTP/1.1\r\n",
            &tunnel.remote_host, tunnel.remote_port
        );
        if let Some(u) = user {
            s.push_str(&format!("Proxy-Authorization: Basic {}\r\n", u));
        };
        s.push_str("\r\n");
        stream.write_all(s.as_bytes()).await?;
        Ok(())
    }

    async fn read_proxy_response(stream: &mut TcpStream) -> io::Result<()> {
        let mut header = [0; 12];
        stream.read_exact(&mut header).await?;

        // check status code of proxy response
        let result_code = match ::std::str::from_utf8(&header) {
            Err(_) => return Err(other_error("Invalid header - not UTF8")),
            Ok(s) => match str::parse::<u16>(&s[9..12]) {
                Ok(n) => n,
                Err(_) => return Err(other_error("Invalid status - not number")),
            },
        };

        if result_code < 200 || result_code >= 300 {
            return Err(other_error(&format!("Invalid status - {}", result_code)));
        }

        let mut status = Status::HeaderOk;

        loop {
            let mut next_byte = [0; 1];
            stream.read_exact(&mut next_byte).await?;

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

        Ok(())
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
mod tests {}
