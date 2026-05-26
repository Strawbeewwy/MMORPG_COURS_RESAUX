mod app;
mod config;
mod net;
mod pubsub;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::run().await
}