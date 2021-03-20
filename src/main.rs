use std::net::SocketAddr;

use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let mut addr = SocketAddr::new("0.0.0.0".parse().unwrap(),8080);
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind port");
    while let Ok((socks, addr)) = listener.accept().await {
        handle_client(socks);    
    }
}

async fn handle_client(peer_left: TcpStream) {
    Client::from_socket(socks);
}