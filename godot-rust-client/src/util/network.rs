use anyhow::Result;
use std::{net::SocketAddr, sync::Arc, time::Duration};

use quinn::{ClientConfig, Endpoint, TransportConfig};

static SERVER_NAME: &str = "localhost";

fn client_addr() -> SocketAddr {
    "127.0.0.1:5000".parse::<SocketAddr>().unwrap()
}

fn server_addr() -> SocketAddr {
    "127.0.0.1:5001".parse::<SocketAddr>().unwrap()
}

// Implementation of `ServerCertVerifier` that verifies everything as trustworthy.
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

pub async fn connect() -> Result<quinn::Connection> {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    let mut config = ClientConfig::new(Arc::new(crypto));

    let mut transport_config = TransportConfig::default();
    // Server has a 10 sec timeout
    transport_config.keep_alive_interval(Some(Duration::from_secs(2)));
    transport_config.max_idle_timeout(Some(Duration::from_secs(10).try_into()?));

    config.transport_config(Arc::new(transport_config));

    // Bind this endpoint to a UDP socket on the given client address.
    let mut endpoint = Endpoint::client(client_addr())?;
    endpoint.set_default_client_config(config);

    // Connect to the server passing in the server name which is supposed to be in the server certificate.
    let connection = endpoint.connect(server_addr(), SERVER_NAME)?.await?;

    return Ok(connection);
}
