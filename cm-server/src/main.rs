mod actors;

use ractor::Actor;
use tracing::info;
use tracing_subscriber;

use crate::actors::server::ServerActor;

#[tokio::main]
async fn main() {
    tracing::subscriber::set_global_default(tracing_subscriber::FmtSubscriber::builder().finish())
        .unwrap();

    info!("Starting server!");
    let (_actor, actor_handle) = Actor::spawn(Some("Server".to_string()), ServerActor, ())
        .await
        .expect("Server actor failed to start");
    let _ = actor_handle.await;
}
