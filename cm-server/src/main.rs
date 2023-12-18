use std::{error::Error, net::SocketAddr, sync::Arc};

use quinn::Endpoint;

static SERVER_NAME: &str = "localhost";

fn server_addr() -> SocketAddr {
    "127.0.0.1:5001".parse::<SocketAddr>().unwrap()
}

fn generate_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey), Box<dyn Error>>
{
    let cert = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_string()])?;
    let key = rustls::PrivateKey(cert.serialize_private_key_der());
    Ok((rustls::Certificate(cert.serialize_der()?), key))
}

async fn server() -> Result<(), Box<dyn Error>> {
    let (cert, key_der) = generate_self_signed_cert()?;
    let server_crypto = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key_der)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));

    let endpoint = Endpoint::server(server_config, server_addr())?;

    // Start iterating over incoming connections.
    while let Some(conn) = endpoint.accept().await {
        let connection = conn.await?;
        let (mut send, mut recv) = connection.open_bi().await?;

        let received = recv.read_to_end(10).await?;
        // WE HAVE BYTES
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    println!("Starting server!");
    server().await.unwrap();
}
