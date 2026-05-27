use crate::config::ServerConfig;
use crate::net::area_of_interest::DEFAULT_AREA_OF_INTEREST_RADIUS;
use crate::net::input::handle_broker_client_input;
use crate::world::state::{
    PlayerRegistry, handle_add_client_to_shard,
};
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::broker::{
    BrokerMessage, Topic, decode_message, encode_publish,
    encode_register_shard, topic_to_string,
};
use shared::protocol::transport::codec;
use shared::protocol::WorldUpdate;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Resource, Clone)]
pub struct SharedPlayerRegistry {
    pub inner: Arc<Mutex<PlayerRegistry>>,
}

#[derive(Resource)]
pub struct BrokerShardPeer {
    pub peer: GamePeer,
    pub connection: Option<GameConnection>,
    pub reliable_stream: Option<GameStream>,
    pub registered: bool,
}

pub fn connect_to_broker(mut commands: Commands, config: Res<ServerConfig>) {
    let peer = GamePeer::new(QuicBackend::new());

    if let Err(error) = peer.connect(&config.broker_ip, config.broker_port) {
        tracing::error!(
            "failed to connect shard to broker {}: {}",
            config.broker_addr(),
            error
        );
    }

    tracing::info!(
        "shard connecting to broker={} zone={} topic={}",
        config.broker_addr(),
        config.zone,
        topic_to_string(&config.shard_topic)
    );

    commands.insert_resource(BrokerShardPeer {
        peer,
        connection: None,
        reliable_stream: None,
        registered: false,
    });
}

pub fn poll_broker_events(
    config: Res<ServerConfig>,
    mut broker_peer: ResMut<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
) {
    loop {
        let event = match broker_peer.peer.poll() {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(error) => {
                tracing::error!("failed to poll shard broker connection: {error}");
                break;
            }
        };

        handle_broker_event(&config, &mut broker_peer, &registry, event);
    }
}

fn handle_broker_event(
    config: &ServerConfig,
    broker_peer: &mut BrokerShardPeer,
    registry: &SharedPlayerRegistry,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            tracing::info!("shard connected to broker: {}", connection.connection_id);

            broker_peer.connection = Some(connection);

            if let Err(error) = broker_peer
                .peer
                .create_stream(connection, GameStreamReliability::Reliable)
            {
                tracing::error!(
                    "failed to create reliable stream to broker on connection {}: {}",
                    connection.connection_id,
                    error
                );
            }
        }

        GameNetworkEvent::Disconnected(connection) => {
            tracing::warn!("shard disconnected from broker: {}", connection.connection_id);

            broker_peer.connection = None;
            broker_peer.reliable_stream = None;
            broker_peer.registered = false;
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "broker stream created for shard: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            if stream.is_reliable() {
                broker_peer.connection = Some(connection);
                broker_peer.reliable_stream = Some(stream);
                register_shard_with_broker(config, broker_peer);
            }
        }

        GameNetworkEvent::StreamClosed(connection, stream) => {
            tracing::info!(
                "broker stream closed for shard: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            broker_peer.reliable_stream = None;
            broker_peer.registered = false;
        }

        GameNetworkEvent::Message {
            connection,
            stream,
            data,
        } => {
            tracing::debug!(
                "broker message received by shard: connection={} stream={} bytes={}",
                connection.connection_id,
                stream.stream_id,
                data.len()
            );

            handle_broker_message(config, broker_peer, registry, &data);
        }

        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!(
                "broker socket error for shard on connection {}: {}",
                connection.connection_id,
                inner
            );

            broker_peer.connection = None;
            broker_peer.reliable_stream = None;
            broker_peer.registered = false;
        }
    }
}

fn register_shard_with_broker(
    config: &ServerConfig,
    broker_peer: &mut BrokerShardPeer,
) {
    if broker_peer.registered {
        return;
    }

    let packet = encode_register_shard(config.shard_topic);

    if !send_raw_to_broker(broker_peer, packet, "RegisterShard") {
        return;
    }

    broker_peer.registered = true;

    tracing::info!(
        "registered shard with broker topic={}",
        topic_to_string(&config.shard_topic)
    );
}

fn handle_broker_message(
    config: &ServerConfig,
    broker_peer: &BrokerShardPeer,
    registry: &SharedPlayerRegistry,
    data: &[u8],
) {
    let message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode broker message in shard: {error:#}");
            return;
        }
    };

    match message {
        BrokerMessage::AddClientToShard {
            topic,
            client_id,
            payload,
        } => {
            if topic != config.shard_topic {
                tracing::warn!(
                    "received AddClientToShard for wrong topic: got={} expected={}",
                    topic_to_string(&topic),
                    topic_to_string(&config.shard_topic)
                );
                return;
            }

            handle_add_client_to_shard(config, registry, client_id, &payload);
        }

        BrokerMessage::ClientInput { client_id, input } => {
            handle_broker_client_input(config, registry, client_id, input);
        }

        other => {
            tracing::warn!("unexpected broker message received by shard: {:?}", other);
        }
    }
}

pub fn publish_world_snapshots(
    config: Res<ServerConfig>,
    broker_peer: Res<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
) {
    if !broker_peer.registered {
        return;
    }

    let Ok(registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for shard world snapshot publish");
        return;
    };

    let snapshot = registry.snapshot(config.zone.clone());

    let update = WorldUpdate::Snapshot { snapshot };

    publish_world_update(&broker_peer, config.shard_topic, update);
}

fn publish_world_update(
    broker_peer: &BrokerShardPeer,
    topic: Topic,
    update: WorldUpdate,
) {
    let payload = match codec::encode(&update) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!("failed to encode WorldUpdate: {error:#}");
            return;
        }
    };

    let packet = match encode_publish(topic, &payload) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::error!("failed to encode broker Publish: {error:#}");
            return;
        }
    };

    send_raw_to_broker(broker_peer, packet, "Publish");
}

fn send_raw_to_broker(
    broker_peer: &BrokerShardPeer,
    packet: Vec<u8>,
    label: &str,
) -> bool {
    let Some(connection) = broker_peer.connection else {
        tracing::warn!("cannot send {label}: shard is not connected to broker");
        return false;
    };

    let Some(stream) = broker_peer.reliable_stream.as_ref() else {
        tracing::warn!("cannot send {label}: shard reliable stream is not ready");
        return false;
    };

    match broker_peer.peer.send(&connection, stream, Bytes::from(packet)) {
        Ok(()) => true,
        Err(error) => {
            tracing::error!("failed to send {label} to broker: {}", error);
            false
        }
    }
}