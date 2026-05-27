use bevy::prelude::*;
use game_sockets::protocols::QuicBackend;
use game_sockets::GamePeer;
use crate::resources::config::SpatialConfig;
use crate::resources::net_handles::BrokerClient;

/// Startup system — initiate the QUIC connection towards the broker.
/// The handshake completes asynchronously; `BrokerClient::is_ready()` turns true
/// once `poll_broker_connection` processes the Connected + StreamCreated events.
pub fn connect_to_broker(mut commands: Commands, config: Res<SpatialConfig>) {
    let peer = GamePeer::new(QuicBackend::new());

    if let Err(e) = peer.connect(&config.broker_host, config.broker_port) {
        tracing::error!(
            "spatial: failed to start connection to broker {}:{}: {e}",
            config.broker_host,
            config.broker_port
        );
    } else {
        tracing::info!(
            "spatial: connecting to broker at {}:{}",
            config.broker_host,
            config.broker_port
        );
    }

    commands.insert_resource(BrokerClient {
        peer,
        connection: None,
        stream: None,
    });
}

