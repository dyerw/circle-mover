use anyhow::Result;
use prost::{decode_length_delimiter, length_delimiter_len};
use quinn::Endpoint;
use std::{error::Error, net::SocketAddr, sync::Arc};
use tokio::io::AsyncReadExt;

use cm_protos::{
    cm_proto::messages::{circle_mover_message::Value, CircleMoverMessage},
    deserialize_message,
};

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

#[tokio::main]
async fn main() {
    println!("Starting server!");
    server().await.unwrap();
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
        let connection_fut = handle_connection(conn);
        // Spawn a task for each connection
        tokio::spawn(connection_fut);
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Connecting) -> Result<()> {
    let connection = conn.await?;

    println!("waiting to accept bi");
    let stream = connection.accept_bi().await;
    let (mut _send, mut recv) = match stream {
        Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
            println!("Connection Error");
            return Ok(());
        }
        Err(e) => {
            println!("{:?}", e);
            return Err(e.into());
        }
        Ok(s) => s,
    };
    println!("Bi accepted");

    loop {
        // FIXME: This will bug out if receiving a message < 9 bytes
        // Length delimeter is at most 10 bytes
        let mut len_delimeter_buf = [0u8; 10];
        println!("Waiting for 10 bytes");
        recv.read_exact(&mut len_delimeter_buf).await?;
        println!("Got 10 bytes");
        // Use the 10 bytes to determine the length of the next message
        let len_delimeter = decode_length_delimiter(&len_delimeter_buf[..])?;
        // Determine how big the length delimeter is, we now need to read len_delimeter - (10 - delim_len) more bytes
        // to get our message
        let delim_len = length_delimiter_len(len_delimeter);

        let rest_len = len_delimeter - (10 - delim_len);

        println!("Waiting for {} more bytes", rest_len);
        let mut rest_buf = vec![0u8; len_delimeter - (10 - delim_len)];
        recv.read_exact(&mut rest_buf).await?;
        println!("Got em");

        let msg_buf = [&len_delimeter_buf[delim_len..], &rest_buf[..]].concat();
        println!("Message buf: {:?}", msg_buf);

        match deserialize_message(&msg_buf) {
            Ok(msg) => match msg {
                CircleMoverMessage {
                    value: Some(Value::Hello(hello_msg)),
                } => {
                    println!("Hello {}", hello_msg.name)
                }
                CircleMoverMessage {
                    value: Some(Value::Goodbye(goodbye_msg)),
                } => {
                    println!("Goodbye {}", goodbye_msg.name)
                }
                CircleMoverMessage { value: None } => {
                    println!("Empty value")
                }
            },
            Err(e) => {
                println!("{:?}", e)
            }
        }
    }
}
