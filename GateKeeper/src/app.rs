use crate::net;
use anyhow::Result;

pub async fn run() -> Result<()> {
    net::server::run().await
}