use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;

use quinn::{Endpoint, TransportConfig};
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tracing::{error, info};

use super::{
    connection::{ConnectionActor, ConnectionArguments, ConnectionMessage},
    lobby::{LobbyActor, LobbyArguments, LobbyMessage},
};

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
    CreateLobby {
        name: String,
        host: ActorRef<ConnectionMessage>,
    },
}

pub struct ServerState {
    connection_actors: Vec<ActorRef<ConnectionMessage>>,
    lobbies: HashMap<String, ActorRef<LobbyMessage>>,
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
        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));

        let mut transport_config = TransportConfig::default();
        transport_config.max_idle_timeout(Some(Duration::from_secs(10).try_into()?));
        transport_config.keep_alive_interval(Some(Duration::from_secs(2)));

        server_config.transport_config(Arc::new(transport_config));

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
            lobbies: HashMap::new(),
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ServerMessage::NewConnection(conn) => {
                let (actor, _actor_handle) = Actor::spawn(
                    None,
                    ConnectionActor,
                    ConnectionArguments {
                        server_ref: myself,
                        connecting: conn,
                    },
                )
                .await
                .expect("Connection actor failed to start");
                state.connection_actors.push(actor);
            }
            ServerMessage::CreateLobby { name, host } => {
                let name_key = name.clone();
                let name_for_lobby = name.clone();
                let (actor, _) = Actor::spawn(
                    Some(name),
                    LobbyActor,
                    LobbyArguments {
                        name: name_for_lobby,
                        host_conn: host,
                    },
                )
                .await
                .expect("Failed to start lobby actor");
                state.lobbies.insert(name_key, actor);
            }
        }
        Ok(())
    }
}
