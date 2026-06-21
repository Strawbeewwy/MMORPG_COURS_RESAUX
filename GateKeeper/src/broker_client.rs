use std::sync::Arc;
use anyhow::{Context, Result};
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{GameNetworkEvent, GamePeer, GameStreamReliability};
use shared::protocol::{decode_message, encode_message, ClientId, NetworkMessage};
use std::time::{Duration, Instant};

pub fn register_client_with_broker(
    broker_host: &str,
    broker_port: u16,
    username: &str,
) -> Result<ClientId> {
    let mut peer = GamePeer::new(QuicBackend::new());

    peer.connect(broker_host, broker_port)
        .with_context(|| format!("failed to connect to Broker at {broker_host}:{broker_port}"))?;

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut connection = None;
    let mut stream = None;
    let mut client_hello_sent = false;

    while Instant::now() < deadline {
        match peer.poll().context("failed to poll Broker connection")? {
            Some(GameNetworkEvent::Connected(conn)) => {
                peer.create_stream(conn, GameStreamReliability::Reliable)
                    .context("failed to create reliable stream to Broker")?;

                connection = Some(conn);
            }
            Some(GameNetworkEvent::StreamCreated(conn, created_stream)) => {
                connection = Some(conn);
                stream = Some(created_stream);
            }
            Some(GameNetworkEvent::Message { data, .. }) => {
                let message = decode_message(&data)
                    .context("failed to decode Broker response")?;

                if let NetworkMessage::ClientAccepted { client_id } = message {
                    tracing::info!(
                        "GateKeeper registered client username={} client_id={}",
                        username,
                        client_id.0
                    );

                    return Ok(client_id);
                }
            }
            Some(GameNetworkEvent::Disconnected(conn)) => {
                anyhow::bail!(
                    "Broker disconnected while registering client: {}",
                    conn.connection_id
                );
            }
            Some(GameNetworkEvent::StreamClosed(_, _)) => {
                anyhow::bail!("Broker stream closed while registering client");
            }
            Some(GameNetworkEvent::Error { inner, .. }) => {
                anyhow::bail!("Broker connection error while registering client: {inner}");
            }
            None => {}
        }

        if !client_hello_sent {
            if let (Some(conn), Some(created_stream)) = (connection.as_ref(), stream.as_ref()) {
                let packet = encode_message(&NetworkMessage::ClientHello {
                    username: Arc::from(username.to_string()),
                })
                    .context("failed to encode ClientHello")?;

                peer.send(conn, created_stream, packet.into())
                    .context("failed to send ClientHello to Broker")?;

                client_hello_sent = true;
            }
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    anyhow::bail!("timed out waiting for Broker ClientAccepted response")
}