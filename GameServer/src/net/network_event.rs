use crate::config::ServerConfig;
use crate::net::input::handle_broker_client_input;
use crate::world::state::{EntityRegistry, handle_register_client};
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use shared::protocol::{NetworkMessage, Topic, decode_message, encode_message, BrokerHandle, BrokerConnectionState, ClientId, NetVec2, ZoneId, PlayerSnapshot, WorldSnapshot};
use shared::protocol::transport::codec;
use shared::protocol::WorldUpdate;
use std::sync::Arc;
use bevy::platform::collections::HashMap;
use tokio::sync::Mutex;
use shared::game_sockets::GameNetworkEvent::{Connected, Disconnected, StreamClosed, StreamCreated};

#[derive(Resource, Clone)]
pub struct SharedPlayerRegistry {
    pub inner: Arc<Mutex<EntityRegistry>>,
}

#[derive(Resource, Default)]
pub struct PublishedPlayerPositions {
    positions_by_client: HashMap<ClientId, NetVec2>,
}

#[derive(Resource)]
pub struct BrokerShardPeer {
    handle: BrokerHandle,
}

impl BrokerShardPeer{
    pub fn new(handle: BrokerHandle) -> Self {
        Self { handle }
    }


    pub fn is_ready(&self) -> bool {
        self.handle.is_ready()
    }

    pub fn send_message(&self, message: &NetworkMessage) -> anyhow::Result<()> {
        let packet = encode_message(message)?;
        self.handle.send(packet)
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
            tracing::warn!("unexpected broker message received by shard: {:?}", other);
        }
    }
}

pub fn generate_world_snapshots(
    config: &Res<ServerConfig>,
    broker: &Res<BrokerShardPeer>,
    registry: &Res<SharedPlayerRegistry>,
) -> anyhow::Result<WorldUpdate>{
    if !broker.is_ready() {
        anyhow::bail!("broker is not ready");
    }

    let Ok(registry) = registry.inner.try_lock() else {
        anyhow::bail!("Could not lock player registry");
    };

    let player_snapshot = registry.generate_player_snapshot();


    let world_snapshot = WorldSnapshot {
        zone: config.zone.clone(),
        players: player_snapshot,
        server_tick: config.server_tick.clone(),
    };


    Ok(WorldUpdate::Snapshot { snapshot: world_snapshot })

}


pub fn publish_player_position_updates(
    broker: Res<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
    mut published_positions: ResMut<PublishedPlayerPositions>,
) {
    if !broker.is_ready() {
        return;
    }

    let Ok(registry) = registry.inner.try_lock() else {
        tracing::warn!("could not lock player registry for position updates");
        return;
    };

    published_positions
        .positions_by_client
        .retain(|client_id, _| registry.client_player.contains_key(client_id));

    for (client_id, player_id) in registry.client_player.iter() {
        let Some(player) = registry.players.get(player_id) else {
            tracing::warn!(
                "cannot publish position update: player not found for client_id={}",
                client_id.0
            );
            continue;
        };

        let position = player.position.clone();

        if published_positions.positions_by_client.get(client_id) == Some(&position) {
            continue;
        }

        let message = NetworkMessage::PositionUpdate {
            client_id: *client_id,
            position: position.clone(),
        };

        if let Err(error) = broker.send_message(&message) {
            tracing::error!(
                "failed to publish position update for client_id={}: {error:#}",
                client_id.0
            );
            return;
        }

        published_positions
            .positions_by_client
            .insert(*client_id, position);
    }
}

pub fn publish_world_update(
    broker: Res<BrokerShardPeer>,
    registry: Res<SharedPlayerRegistry>,
    config: Res<ServerConfig>,
) {
    let topic = config.shard_topic;

    let update = match generate_world_snapshots(&config, &broker, &registry) {
        Ok(update) => update,
        Err(error) => {
            tracing::error!("failed to generate world snapshots: {error:#}");
            return;
        }
    };

    let payload = match codec::encode(&update) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!("failed to encode WorldUpdate: {error:#}");
            return;
        }
    };

    let payload_len = match u16::try_from(payload.len()) {
        Ok(payload_len) => payload_len,
        Err(_) => {
            tracing::error!("WorldUpdate payload too large: {} bytes", payload.len());
            return;
        }
    };

    let Topic::ShardInstance(shard_id) = topic else {
        tracing::warn!("cannot publish WorldUpdate to unsupported topic {}", topic.to_string());
        return;
    };

    let packet = match encode_message(&NetworkMessage::Publish {
        shard_id,
        payload_len,
        payload,
    }) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::warn!(
                "cannot encode Publish for topic {}: {}",
                topic.to_string(),
                error
            );
            return;
        }
    };

    if let Err(error) = broker.handle.send(packet) {
        tracing::error!("failed to send packet to broker: {error:#}");
    }
}