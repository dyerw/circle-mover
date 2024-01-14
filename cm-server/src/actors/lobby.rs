use std::time::{Duration, SystemTime};

use anyhow::Result;
use cm_sim::{
    actor::{SimActor, SimArguments, SimMessage},
    game::Game,
};
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tokio::sync::watch;
use tracing::info;

use super::connection::ConnectionMessage;

pub enum LobbyMessage {
    AddPlayer(ActorRef<ConnectionMessage>),
    RequestStartGame,
}

pub struct LobbyState {
    name: String,
    host_conn: ActorRef<ConnectionMessage>,
    player_conns: Vec<ActorRef<ConnectionMessage>>,
    sim: Option<ActorRef<SimMessage>>,
    game_state_receiver: Option<watch::Receiver<(i32, Game)>>,
}

pub struct LobbyArguments {
    pub name: String,
    pub host_conn: ActorRef<ConnectionMessage>,
}

pub struct LobbyActor;

#[async_trait]
impl Actor for LobbyActor {
    type Msg = LobbyMessage;
    type State = LobbyState;
    type Arguments = LobbyArguments;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("Creating lobby");
        let state_name = arguments.name.clone();
        arguments.host_conn.cast(ConnectionMessage::JoinedLobby {
            name: arguments.name,
            lobby_ref: myself,
        })?;
        Ok(LobbyState {
            name: state_name,
            host_conn: arguments.host_conn,
            player_conns: vec![],
            sim: None,
            game_state_receiver: None,
        })
    }

    async fn handle(
        &self,
        myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            LobbyMessage::AddPlayer(player_conn) => {
                if let Ok(()) = player_conn.cast(ConnectionMessage::JoinedLobby {
                    lobby_ref: myself,
                    name: state.name.clone(),
                }) {
                    state.player_conns.push(player_conn);
                }
            }
            LobbyMessage::RequestStartGame => {
                let (state_tx, state_rx) =
                    watch::channel((0, Game::new(Duration::from_millis(22))));
                let (actor, _) = Actor::spawn(
                    None,
                    SimActor,
                    SimArguments {
                        minimum_tick_duration: Duration::from_millis(22),
                        game_state_sender: state_tx,
                    },
                )
                .await
                .expect("Failed to start sim");

                // Synchronize start for all clients
                let start_at = SystemTime::now() + Duration::from_secs(5);
                state
                    .host_conn
                    .cast(ConnectionMessage::SendSynchronizedGameStart(start_at))?;
                for c in state.player_conns.iter() {
                    c.cast(ConnectionMessage::SendSynchronizedGameStart(start_at))?;
                }
                // Synchronize server sim
                actor.cast(SimMessage::StartAt(start_at))?;
                state.sim = Some(actor);
                state.game_state_receiver = Some(state_rx);
            }
        };
        Ok(())
    }
}
