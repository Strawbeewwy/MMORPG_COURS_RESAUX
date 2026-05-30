use bevy::prelude::*;
use game_sockets::protocols::QuicBackend;
use game_sockets::GamePeer;
use std::time::{Duration, Instant};
use crate::resources::config::SpatialConfig;
use crate::resources::net_handles::{BrokerClient, BrokerConnectionState};

/// Startup system — initiate the QUIC connection towards the broker.
/// The handshake completes asynchronously; state advances to `Ready` once
/// `poll_broker_connection` processes Connected + StreamCreated events.
/// On failure the resource is inserted with `Disconnected` state so that
/// `reconnect_broker_if_needed` retries automatically.
pub fn connect_to_broker(mut commands: Commands, config: Res<SpatialConfig>) {
    let peer = GamePeer::new(QuicBackend::new());
    let state = match peer.connect(&config.broker_host, config.broker_port) {
        Ok(_) => {
            tracing::info!(
                "spatial: connecting to broker at {}:{}",
                config.broker_host, config.broker_port
            );
            BrokerConnectionState::Connecting
        }
        Err(e) => {
            tracing::error!(
                "spatial: failed to start connection to broker {}:{}: {e}",
                config.broker_host, config.broker_port
            );
            BrokerConnectionState::Disconnected
        }
    };
    commands.insert_resource(BrokerClient::with_state(peer, state));
}

/// Reconnect system — called each tick when the broker connection is lost.
/// Uses exponential backoff (1 s → 2 s → 4 s … capped at 30 s) to avoid
/// flooding QUIC with reconnect attempts after a drop.
pub fn reconnect_broker_if_needed(
    mut broker: ResMut<BrokerClient>,
    config: Res<SpatialConfig>,
) {
    if broker.state != BrokerConnectionState::Disconnected {
        return;
    }

    // Honour the backoff window — skip this tick if too early.
    if let Some(after) = broker.reconnect_after {
        if Instant::now() < after {
            return;
        }
    }

    tracing::info!(
        "broker disconnected — reconnect attempt #{} to {}:{}",
        broker.reconnect_attempt + 1,
        config.broker_host,
        config.broker_port
    );

    broker.reset_for_reconnect();

    if let Err(e) = broker.peer.connect(&config.broker_host, config.broker_port) {
        tracing::error!("reconnect to broker failed: {e}");
        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, capped at 30s.
        let delay_secs = (1u64 << broker.reconnect_attempt.min(5)).min(30);
        broker.reconnect_after = Some(Instant::now() + Duration::from_secs(delay_secs));
        broker.reconnect_attempt = broker.reconnect_attempt.saturating_add(1);
        broker.state = BrokerConnectionState::Disconnected;
    }
    // On success, backoff is reset once the `Ready` state is reached
    // (in `poll_broker_connection` via `broker.reset_backoff()`).
}
