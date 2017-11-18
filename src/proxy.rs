use futures::{future, Future, Stream, Poll};
use tokio_io::{io, AsyncRead, AsyncWrite};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Handle;
use tokio_core::net::TcpStream;
use std::net::{SocketAddr, Shutdown};
use config::Tunnel;
use tokio_dns::tcp_connect;
use std::sync::Arc;
use std::io::{Read, Write, Result as IoResult, Error as IoError};

#[derive(Clone)]
struct FixedTcpStream(Arc<TcpStream>);

impl From<TcpStream> for FixedTcpStream {
    fn from(s:TcpStream) -> Self {
        FixedTcpStream(Arc::new(s))
    }
}

impl Read for FixedTcpStream {
    fn read(&mut self, buf: &mut [u8]) ->IoResult<usize> {
        (&*self.0).read(buf)
    }
}

impl AsyncRead for FixedTcpStream {

}

impl Write for FixedTcpStream {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        (&* self.0).write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        (&* self.0).flush()
    }
}

impl AsyncWrite for FixedTcpStream {
    fn shutdown(&mut self) -> Poll<(), IoError> {
        self.0.shutdown(Shutdown::Write)?;
        Ok(().into())
    }
}

pub fn run_tunnel(
    reactor_handle: Handle,
    tunnel: Tunnel,
) -> Box<Future<Item = (), Error = ::std::io::Error>> {
    // Bind the server's socket
    let addr = SocketAddr::new("127.0.0.1".parse().unwrap(), tunnel.local_port);
    let tcp = match TcpListener::bind(&addr, &reactor_handle) {
        Ok(l) => l,
        Err(e) => return Box::new(future::err(e)),
    };

    // Iterate incoming connections
    let server = tcp.incoming().for_each(move |(tcp, client_addr)| {
        debug!("Client connected from {}", client_addr);
        // Split up the read and write halves
        

        let reactor_handle2 = reactor_handle.clone();
        let tunnel2 = tunnel.clone();
        let remote = tcp_connect(
            (&tunnel.remote_host[..], tunnel.remote_port),
            reactor_handle.remote().clone(),
        ).map_err(move |e| {
            error!(
                "cannot connect remote end {} because of error {}",
                tunnel2.remote(),
                e
            );
            // TODO: Close connection
        })
            .and_then(move |remote_socket| {
                debug!("Connected to remote host {:?}", remote_socket);
                let reader  = FixedTcpStream::from(tcp);
                let writer = reader.clone();

                let remote_reader = FixedTcpStream::from(remote_socket);
                let remote_writer = remote_reader.clone();

                // Future of the copy
                let copy_forward = io::copy(reader, remote_writer)
                    .and_then(|(n, _, writer)| {
                        io::shutdown(writer).map(move |_| n)
                    });

                let copy_backward = io::copy(remote_reader, writer)
                    .and_then(|(n, _, writer)| {
                        io::shutdown(writer).map(move |_| n)
                    });

                let copy_both = copy_forward.join(copy_backward)
                    .map(|(up,down)| debug!("Uploaded {} bytes and downloaded {} bytes", up, down))
                    .map_err(|e| warn!("Tunnel connection error {}",e));

                // Spawn the future as a concurrent task
                reactor_handle2.spawn(copy_both);
                Ok(())
            });
        reactor_handle.spawn(remote);
        Ok(())
    });

    Box::new(server)
}
