use bevy::prelude::*;
use game_sockets::protocols::QuicBackend;
use game_sockets::GamePeer;
use crate::resources::config::SpatialConfig;
use crate::resources::net_handles::{BrokerClient, BrokerConnectionState};

/// Startup system — initiate the QUIC connection towards the broker.
/// The handshake completes asynchronously; state advances to `Ready` once
/// `poll_broker_connection` processes Connected + StreamCreated events.
pub fn connect_to_broker(mut commands: Commands, config: Res<SpatialConfig>) {
    let peer = GamePeer::new(QuicBackend::new());
    try_connect(&peer, &config);
    commands.insert_resource(BrokerClient::new(peer));
}

/// Reconnect system — called each tick when the broker connection is lost.
/// Re-creates the outbound QUIC attempt without restarting the whole service.
pub fn reconnect_broker_if_needed(
    mut broker: ResMut<BrokerClient>,
    config: Res<SpatialConfig>,
) {
    if broker.state != BrokerConnectionState::Disconnected {
        return;
    }

    tracing::info!("broker disconnected — attempting reconnect to {}:{}", config.broker_host, config.broker_port);
    broker.reset_for_reconnect();

    // Attempt a new connection on the existing peer (game_sockets allows re-connecting).
    if let Err(e) = broker.peer.connect(&config.broker_host, config.broker_port) {
        tracing::error!("reconnect to broker failed: {e}");
    }
}

fn try_connect(peer: &GamePeer, config: &SpatialConfig) {
    if let Err(e) = peer.connect(&config.broker_host, config.broker_port) {
        tracing::error!(
            "spatial: failed to start connection to broker {}:{}: {e}",
            config.broker_host, config.broker_port
        );
    } else {
        tracing::info!("spatial: connecting to broker at {}:{}", config.broker_host, config.broker_port);
    }
}

