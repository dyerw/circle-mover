use std::{error::Error, net::SocketAddr};

use quinn::{Connection, Endpoint};

static SERVER_NAME: &str = "localhost";

fn client_addr() -> SocketAddr {
    "127.0.0.1:5000".parse::<SocketAddr>().unwrap()
}

fn server_addr() -> SocketAddr {
    "127.0.0.1:5001".parse::<SocketAddr>().unwrap()
}

pub async fn client() -> Result<Connection, Box<dyn Error>> {
    // Bind this endpoint to a UDP socket on the given client address.
    let endpoint = Endpoint::client(client_addr())?;

    // Connect to the server passing in the server name which is supposed to be in the server certificate.
    let connection = endpoint.connect(server_addr(), SERVER_NAME)?.await?;

    Ok(connection)
}
