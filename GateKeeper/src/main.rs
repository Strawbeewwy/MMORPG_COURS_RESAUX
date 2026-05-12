mod server;
mod protocol;
mod config;


use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    server::run().await
}