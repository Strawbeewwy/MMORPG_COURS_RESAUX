use crate::config::ServerConfig;
use crate::net::input::handle_broker_client_input;
use crate::world::state::{PlayerRegistry, handle_add_client_to_shard, handle_register_client};
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::{NetworkMessage, Topic, decode_message, encode_message, BrokerHandle, BrokerConnectionState};
use shared::protocol::transport::codec;
use shared::protocol::WorldUpdate;
use std::sync::Arc;
use tokio::sync::Mutex;
use shared::game_sockets::GameNetworkEvent::{Connected, Disconnected, StreamClosed, StreamCreated};

#[derive(Resource, Clone)]
pub struct SharedPlayerRegistry {
    pub inner: Arc<Mutex<PlayerRegistry>>,
}

#[derive(Resource)]
pub struct BrokerShardPeer {
    handle: BrokerHandle,
}

impl BrokerShardPeer{
    pub fn new(handle: BrokerHandle) -> Self {
        Self { handle }
    }
}

pub fn connect_to_broker(mut commands: Commands, config: Res<ServerConfig>) {
    let peer = GamePeer::new(QuicBackend::new());
    let state = match peer.connect(&config.broker_ip, config.broker_port) {
        Ok(_) => {
            tracing::info!(
                "spatial: connecting to utils at {}:{}",
                config.broker_ip, config.broker_port
            );
            BrokerConnectionState::Connecting
        }
        Err(e) => {
            tracing::error!(
                "spatial: failed to start connection to utils {}:{}: {e}",
                config.broker_ip, config.broker_port
            );
            BrokerConnectionState::Disconnected
        }
    };
    let handle = BrokerHandle::with_state(peer,state);

    commands.insert_resource(BrokerShardPeer::new(handle));
}

pub fn poll_broker_events(
    config: Res<ServerConfig>,
    mut broker: ResMut<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
) {
    loop {
        let event = match broker.handle.peer.poll() {
            Ok(Some(event)) => event,
            Ok(None) => break,
            Err(error) => {
                tracing::error!("failed to poll shard utils connection: {error}");
                break;
            }
        };

        handle_broker_event(&config, &mut broker, &registry, event);
    }
}

fn handle_broker_event(
    config: &ServerConfig,
    broker: &mut BrokerShardPeer,
    registry: &SharedPlayerRegistry,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            tracing::info!("shard connected to utils: {}", connection.connection_id);

            broker.handle.connection = Some(connection);
        broker.handle.state = BrokerConnectionState::Connected;

            if let Err(error) = broker
                .handle
                .peer
                .create_stream(connection, GameStreamReliability::Reliable)
            {
                tracing::error!(
                    "failed to create reliable stream to utils on connection {}: {}",
                    connection.connection_id,
                    error
                );
            }
        }

        GameNetworkEvent::Disconnected(connection) => {
            tracing::warn!("shard disconnected from utils: {}", connection.connection_id);

            broker.handle.connection = None;
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "utils stream created for shard: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

        broker.handle.stream = Some(stream);
        broker.handle.state = BrokerConnectionState::Ready;
        broker.handle.reset_backoff();

        }

        GameNetworkEvent::StreamClosed(connection, stream) => {
            tracing::info!(
                "utils stream closed for shard: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }

        GameNetworkEvent::Message {
            connection,
            stream,
            data,
        } => {
            tracing::debug!(
                "utils message received by shard: connection={} stream={} bytes={}",
                connection.connection_id,
                stream.stream_id,
                data.len()
            );

            handle_broker_message(config, registry, &data);
        }

        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!(
                "utils socket error for shard on connection {}: {}",
                connection.connection_id,
                inner
            );

            broker.handle.connection = None;
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }
    }
}

fn register_shard_with_broker(
    config: &ServerConfig,
    broker: &mut BrokerShardPeer,
) {
    if broker.handle.state == BrokerConnectionState::Connected {
        return;
    }

    if let Topic::ShardInstance(shard_id) = config.shard_topic {
        let packet = match encode_message(&NetworkMessage::RegisterShard {
           shard_id,
        }) {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!(
                "cannot encode RegisterShard for topic {}: {}",
                &config.shard_topic.to_string(),
                error
            );
                return;
            }
        };

        if let Err(error) = broker.handle.send(packet) {
            tracing::error!("failed to send packet to broker: {error:#}");
            return;
        }
    }

    broker.handle.state = BrokerConnectionState::Connected;

    tracing::info!(
        "registered shard with utils topic={}",
        &config.shard_topic.to_string()
    );
}

fn handle_broker_message(
    config: &ServerConfig,
    registry: &SharedPlayerRegistry,
    data: &[u8],
) {
    let message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode utils message in shard: {error:#}");
            return;
        }
    };

    match message {

        NetworkMessage::ClientInput { client_id, input } => {
            handle_broker_client_input(config, registry, client_id, input);
        }
        NetworkMessage::RegisterClient {client_id, username} => {
            handle_register_client(config, registry,client_id, username.clone());
        }

        other => {
            tracing::warn!("unexpected utils message received by shard: {:?}", other);
        }
    }
}

pub fn publish_world_snapshots(
    config: Res<ServerConfig>,
    broker: Res<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
) {
    if broker.handle.state != BrokerConnectionState::Connected {
        return;
    }

    let Ok(registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for shard world snapshot publish");
        return;
    };

    let snapshot = registry.snapshot(config.zone.clone());

    let update = WorldUpdate::Snapshot { snapshot };

    publish_world_update(&broker, config.shard_topic, update);
}

fn publish_world_update(
    broker: &BrokerShardPeer,
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

    if let Topic::ShardInstance(shard_id) = topic {
        let packet = match encode_message(&NetworkMessage::Publish {
            shard_id,
            payload_len:payload.len() as u16,
            payload: Vec::from(payload),
        }) {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!(
                "cannot encode RegisterShard for topic {}: {}",
                &topic.to_string(),
                error
            );
                return;
            }
        };

        if let Err(error) = broker.handle.send(packet) {
            tracing::error!("failed to send packet to broker: {error:#}");
            return;
        }
    }
}