mod app;
mod auth;
mod config;
mod net;
mod protocol;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}