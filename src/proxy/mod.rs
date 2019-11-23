use futures::{
    try_join, 
    FutureExt};
use tokio;
use tokio::net::{TcpListener, TcpStream};
use std::net::SocketAddr;
use crate::config::Tunnel;
use connector::ProxyConnector;

mod connector;



pub async fn run_tunnel(
    local_addr: ::std::net::IpAddr,
    tunnel: Tunnel,
    remote_socket_addr: SocketAddr,
    proxy: Option<SocketAddr>,
    user: Option<String>
) -> Result<(),::std::io::Error> {
    // Bind the server's socket
    let addr = SocketAddr::new(local_addr, tunnel.local_port);
    let mut listener = TcpListener::bind(&addr).await?;

    
    // Iterate incoming connections

    loop {
        match listener.accept().await {
            Ok((socket, client_addr)) => {
                debug!("Client connected from {}", client_addr);
                let tunnel2 = tunnel.clone();
                tokio::spawn(
                    process_connection(socket, tunnel.clone(), remote_socket_addr, proxy, user.clone())
                    .map(move |r| 
                    if let Err(e) = r {
                        error!("Error in tunnel {:?}: {}", tunnel2, e)
                    } 
                    )
                
                );

            }
            Err(e) => error!("Incoming connection error {}", e)
        }
    }
    
        
}


async fn process_connection(mut socket: TcpStream, 
    tunnel: Tunnel,
    remote_socket_addr: std::net::SocketAddr,
    proxy: Option<SocketAddr>,
    user: Option<String>
    ) -> std::io::Result<()>{

    let mut remote_socket = ProxyConnector::connect(
            remote_socket_addr,
            tunnel.clone(),
            proxy,
            user.clone()
        ).await?;
            
    debug!("Created upstream {:?}", remote_socket);
    
    let (mut ri, mut wi) = socket.split();
    let (mut ro, mut wo) = remote_socket.split();

    let client_to_server = tokio::io::copy(&mut ri, &mut wo);
    let server_to_client = tokio::io::copy(&mut ro, &mut wi);

    try_join!(client_to_server, server_to_client)?;
    debug!("Connection closed {:?}", wi);

    Ok(())
        
}
