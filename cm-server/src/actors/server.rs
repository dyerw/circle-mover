use std::{net::SocketAddr, sync::Arc};

use anyhow::Result;

use quinn::Endpoint;
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tracing::{error, info};

use super::connection::{ConnectionActor, ConnectionMessage};

static SERVER_NAME: &str = "localhost";

fn server_addr() -> SocketAddr {
    "127.0.0.1:5001".parse::<SocketAddr>().unwrap()
}

fn generate_self_signed_cert() -> Result<(rustls::Certificate, rustls::PrivateKey)> {
    let cert = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_string()])?;
    let key = rustls::PrivateKey(cert.serialize_private_key_der());
    Ok((rustls::Certificate(cert.serialize_der()?), key))
}

pub enum ServerMessage {
    NewConnection(quinn::Connecting),
}

pub struct ServerState {
    connection_actors: Vec<ActorRef<ConnectionMessage>>,
}

pub struct ServerActor;

#[async_trait]
impl Actor for ServerActor {
    type State = ServerState;
    type Msg = ServerMessage;
    type Arguments = ();

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        let (cert, key_der) = generate_self_signed_cert()?;
        let server_crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(vec![cert], key_der)?;
        let server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));

        let endpoint = Endpoint::server(server_config, server_addr())?;

        tokio::spawn(async move {
            info!("Accepting connections");
            while let Some(conn) = endpoint.accept().await {
                if let Err(e) = myself.cast(ServerMessage::NewConnection(conn)) {
                    error!("Failed: {reason}", reason = e.to_string());
                }
            }
        });

        Ok(ServerState {
            connection_actors: vec![],
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ServerMessage::NewConnection(conn) => {
                let (actor, _actor_handle) = Actor::spawn(None, ConnectionActor, conn)
                    .await
                    .expect("Connection actor failed to start");
                state.connection_actors.push(actor);
            }
        }
        Ok(())
    }
}
