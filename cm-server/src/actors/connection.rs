use anyhow::Result;
use cm_protos::{cm_proto::messages::CircleMoverMessage, deserialize_message};
use ractor::{async_trait, Actor, ActorProcessingErr, ActorRef};
use tracing::{error, info, trace, trace_span};

const MAX_MESSAGE_SIZE: usize = 1024;

pub enum ConnectionMessage {
    ReceivedMessage(CircleMoverMessage),
}

pub struct ConnectionState {
    connection: quinn::Connection,
}

pub struct ConnectionActor;

#[async_trait]
impl Actor for ConnectionActor {
    type State = ConnectionState;
    type Msg = ConnectionMessage;
    type Arguments = quinn::Connecting;

    async fn pre_start(
        &self,
        myself: ActorRef<Self::Msg>,
        arguments: Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        info!("Client connecting");
        let connection = arguments.await?;

        // Kick off task to poll the connection for incoming streams from client
        let connection_clone = connection.clone();
        tokio::spawn(async move {
            loop {
                match read_message(&connection_clone).await.and_then(|msg| {
                    myself.cast(ConnectionMessage::ReceivedMessage(msg))?;
                    Ok(())
                }) {
                    Ok(()) => {}
                    Err(e) => {
                        error!("Failed: {reason}", reason = e.to_string());
                    }
                }
            }
        });

        info!("Client connected");
        Ok(ConnectionState { connection })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            ConnectionMessage::ReceivedMessage(msg) => {
                info!("Received message: {:?}", msg);
            }
        }
        Ok(())
    }
}

async fn read_message(conn: &quinn::Connection) -> Result<CircleMoverMessage> {
    let span = trace_span!("Reading message");
    let _enter = span.enter();

    let mut recv = conn.accept_uni().await?;
    let bytes = recv.read_to_end(MAX_MESSAGE_SIZE).await?;

    match deserialize_message(&bytes) {
        Ok(msg) => {
            trace!("Successfully deserialized message");
            return Ok(msg);
        }
        Err(e) => {
            return Err(e.into());
        }
    }
}
