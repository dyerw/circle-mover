use anyhow::Result;
use prost::{decode_length_delimiter, length_delimiter_len};
use quinn::Endpoint;
use std::{error::Error, net::SocketAddr, sync::Arc};
use tracing::{error, info, info_span, trace, trace_span};
use tracing_futures::Instrument;
use tracing_subscriber;

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
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::builder().finish())
        .unwrap();

    info!("Starting server!");
    server().await.unwrap();
}

#[tracing::instrument]
async fn server() -> Result<(), Box<dyn Error>> {
    let (cert, key_der) = generate_self_signed_cert()?;
    let server_crypto = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key_der)?;
    let server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));

    let endpoint = Endpoint::server(server_config, server_addr())?;

    trace!("Accepting connections");
    while let Some(conn) = endpoint.accept().await {
        // Spawn a task for each connection
        tokio::spawn(async {
            if let Err(e) = handle_connection(conn).await {
                error!("Failed: {reason}", reason = e.to_string())
            }
        });
    }

    Ok(())
}

async fn handle_connection(conn: quinn::Connecting) -> Result<()> {
    let connection = conn.await?;
    let span = info_span!(
        "connection",
        remote = %connection.remote_address(),
        protocol = %connection
            .handshake_data()
            .unwrap()
            .downcast::<quinn::crypto::rustls::HandshakeData>().unwrap()
            .protocol
            .map_or_else(|| "<none>".into(), |x| String::from_utf8_lossy(&x).into_owned())
    );
    info!("Connection to client");

    async {
        trace!("Awaiting bidirectional connection");
        let stream = connection.accept_bi().await;
        let (mut _send, mut recv) = match stream {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                info!("Connection closed");
                return Ok(());
            }
            Err(e) => {
                return Err(e.into());
            }
            Ok(s) => s,
        };
        trace!("Bidirectional connection established");

        loop {
            let span = trace_span!("Reading message");
            let _enter = span.enter();
            // FIXME: This will bug out if receiving a message < 9 bytes
            // Length delimeter is at most 10 bytes
            let mut len_delimeter_buf = [0u8; 10];

            trace!("Waiting for 10 bytes");
            recv.read_exact(&mut len_delimeter_buf).await?;
            trace!("Received 10 bytes from client");
            // Use the 10 bytes to determine the length of the next message
            let len_delimeter = decode_length_delimiter(&len_delimeter_buf[..])?;
            // Determine how big the length delimeter is, we now need to read len_delimeter - (10 - delim_len) more bytes
            // to get our message
            let delim_len = length_delimiter_len(len_delimeter);

            let rest_len = len_delimeter - (10 - delim_len);

            trace!("Waiting for more bytes to complete proto message");
            let mut rest_buf = vec![0u8; rest_len];
            recv.read_exact(&mut rest_buf).await?;
            trace!("Received all necessary bites");

            let msg_buf = [&len_delimeter_buf[delim_len..], &rest_buf[..]].concat();
            trace!("Message buffer constructed {:?}", msg_buf);

            match deserialize_message(&msg_buf) {
                Ok(msg) => {
                    info!("Received message: {:?}", msg);
                    match msg {
                        CircleMoverMessage {
                            value: Some(Value::Hello(_hello_msg)),
                        } => {}
                        CircleMoverMessage {
                            value: Some(Value::Goodbye(_goodbye_msg)),
                        } => {}
                        CircleMoverMessage { value: None } => {}
                    }
                }
                Err(e) => {
                    error!("Proto decode error: {:?}", e)
                }
            }
        }
    }
    .instrument(span)
    .await
}
