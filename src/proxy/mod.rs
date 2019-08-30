use futures::{future, Future, Stream};
use tokio_io::io;
use tokio;
use tokio::net::TcpListener;
use std::net::SocketAddr;
use config::{Proxy, Tunnel};
use self::stream::{FixedTcpStream, ProxyTcpStream};

mod stream;



pub fn run_tunnel(
    local_addr: ::std::net::IpAddr,
    tunnel: Tunnel,
    proxy: Option<Proxy>,
    user: Option<String>
) -> Box<dyn Future<Item = (), Error = ::std::io::Error>+Send> {
    // Bind the server's socket
    let addr = SocketAddr::new(local_addr, tunnel.local_port);
    let tcp = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => return Box::new(future::err(e)),
    };

    // Iterate incoming connections
    let server = tcp.incoming().for_each(move |tcp| {
        let client_addr = tcp.peer_addr().unwrap();
        debug!("Client connected from {}", client_addr);
        let tunnel2 = tunnel.clone();
        let remote = ProxyTcpStream::connect(
            tunnel.clone(),
            proxy.as_ref(),
            user.clone()
        ).map_err(move |e| {
            error!(
                "cannot connect remote end {} because of error {}",
                tunnel2.remote(),
                e
            );
            // TODO: Close connection?
        })
            .and_then(move |remote_socket| {
                debug!("Created upstream {:?}", remote_socket);
                let reader = FixedTcpStream::from(tcp);
                let writer = reader.clone();

                let remote_reader = remote_socket;
                let remote_writer = remote_reader.clone();

                let copy_forward = io::copy(reader, remote_writer)
                    .and_then(|(n, _, writer)| io::shutdown(writer).map(move |_| n));

                let copy_backward = io::copy(remote_reader, writer)
                    .and_then(|(n, _, writer)| io::shutdown(writer).map(move |_| n));

                copy_forward
                    .join(copy_backward)
                    .map(|(up, down)| {
                        debug!("Uploaded {} bytes and downloaded {} bytes", up, down)
                    })
                    .map_err(|e| warn!("Tunnel connection error {}", e))
            });
        tokio::spawn(remote);
        Ok(())
    });

    Box::new(server)
}
