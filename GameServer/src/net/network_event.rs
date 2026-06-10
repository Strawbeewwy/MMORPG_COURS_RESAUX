use std::time::{Duration, Instant};
use crate::config::ServerConfig;

use crate::world::state::{SharedEntityRegistry};
use bevy::prelude::*;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameNetworkEvent, GamePeer, GameStreamReliability,
};
use shared::protocol::{
    NetworkMessage, decode_message, encode_message, BrokerHandle, BrokerConnectionState, ClientId,
    NetVec2, WorldSnapshot, WorldUpdate, ShardId
};
use tokio::sync::{MutexGuard};
use crate::net::apply_client_input;
use crate::net::handoff::{
    handle_handoff_start_on_source,
    handle_handoff_request_on_dest,
    handle_handoff_accepted_on_source,
    handle_handoff_rejected_on_source,
};
use crate::world::{EntityIdAllocator, SpawnPlayerEntityEvent, Velocity};
use crate::world::entity::PromoteGhostEvent;
use crate::world::spawn_entity::SpawnGhostEntityEvent;




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

    pub fn send_message_to_broker(&self, message: &NetworkMessage) -> anyhow::Result<()> {
        match encode_message(message) {
            Ok(packet) => {
                if let Err(error) = self.handle.send(packet){
                    return Err(anyhow::anyhow!("failed to send message: {error:#}"));
                }
                Ok(())
            }
            Err(_) => {
                Err(anyhow::anyhow!("failed to encode message"))
            }
        }
    }
}

pub fn connect_to_broker(
    mut commands: Commands,
    config: Res<ServerConfig>
) {
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

pub fn reconnect_broker_if_needed(
    mut broker: ResMut<BrokerShardPeer>,
    config: Res<ServerConfig>,
) {
    if broker.handle.state != BrokerConnectionState::Disconnected {
        return;
    }

    // Honour the backoff window — skip this tick if too early.
    if let Some(after) = broker.handle.reconnect_after {
        if Instant::now() < after {
            return;
        }
    }

    tracing::info!(
        "utils disconnected — reconnect attempt #{} to {}:{}",
        broker.handle.reconnect_attempt + 1,
        config.broker_ip,
        config.broker_port
    );

    broker.handle.reset_for_reconnect();

    if let Err(e) = broker.handle.peer.connect(&config.broker_ip, config.broker_port) {
        tracing::error!("reconnect to utils failed: {e}");
        // Exponential backoff: 1s, 2s, 4s, 8s, 16s, capped at 30s.
        let delay_secs = (1u64 << broker.handle.reconnect_attempt.min(5)).min(30);
        broker.handle.reconnect_after = (Some(Instant::now() + Duration::from_secs(delay_secs)));
        broker.handle.reconnect_attempt = broker.handle.reconnect_attempt.saturating_add(1);
        broker.handle.state = BrokerConnectionState::Disconnected;
    }
    // On success, backoff is reset once the `Ready` state is reached
    // (in `poll_broker_connection` via `utils.reset_backoff()`).
}

pub fn poll_broker_events(
    config: Res<ServerConfig>,
    mut commands: Commands,
    mut broker: ResMut<BrokerShardPeer>,
    mut registry: ResMut<SharedEntityRegistry>,
    mut allocator: ResMut<EntityIdAllocator>,
    mut velocities: Query<&mut Velocity>,
    mut spawn_players: MessageWriter<SpawnPlayerEntityEvent>,
    mut spawn_ghosts: MessageWriter<SpawnGhostEntityEvent>,
    mut promote_ghosts: MessageWriter<PromoteGhostEvent>,
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

        handle_broker_event(
            &config,
            &mut commands,
            &mut broker,
            &mut registry,
            &mut allocator,
            event,
            &mut velocities,
            &mut spawn_players,
            &mut spawn_ghosts,
            &mut promote_ghosts,
        );
    }
}

fn handle_broker_event(
    config: &ServerConfig,
    commands: &mut Commands,
    broker: &mut BrokerShardPeer,
    registry: &mut SharedEntityRegistry,
    allocator: &mut EntityIdAllocator,
    event: GameNetworkEvent,
    velocities: &mut Query<&mut Velocity>,
    spawn_players: &mut MessageWriter<SpawnPlayerEntityEvent>,
    spawn_ghosts: &mut MessageWriter<SpawnGhostEntityEvent>,
    promote_ghosts: &mut MessageWriter<PromoteGhostEvent>,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            info!("shard connected to utils: {}", connection.connection_id);

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
            warn!("shard disconnected from utils: {}", connection.connection_id);

            broker.handle.connection = None;
            broker.handle.stream = None;
            broker.handle.state = BrokerConnectionState::Disconnected;
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            info!(
                "broker stream created for shard: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            broker.handle.stream = Some(stream);
            broker.handle.state = BrokerConnectionState::Ready;
            broker.handle.reset_backoff();


            register_shard_with_broker(config, broker);


            let topic = config.shard_topic;

            let packet = match encode_message(&NetworkMessage::RequestEntityIdBlock{
                shard_id: ShardId(topic.get_id_as_u32()),
                count: config.max_entity,
            }) {
                Ok(packet) => packet,
                Err(error) => {
                    warn!(
                "cannot encode Request EntityId Block for topic {}: {}",
                topic.to_string(),
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

        GameNetworkEvent::StreamClosed(connection, stream) => {
            info!(
                "broker stream closed for shard: connection={} stream={}",
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
            debug!(
                "broker message received: connection={} stream={} bytes={}",
                connection.connection_id,
                stream.stream_id,
                data.len()
            );

            handle_broker_message(
                config,
                commands,
                broker,
                registry,
                allocator,
                &data,
                velocities,
                spawn_players,
                spawn_ghosts,
                promote_ghosts,
            );
        }

        GameNetworkEvent::Error { connection, inner } => {
            warn!(
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
    if !broker.is_ready() {
        return;
    }

    let topic = config.shard_topic;

    let packet = match encode_message(&NetworkMessage::RegisterShard {
        shard_id: ShardId(topic.get_id_as_u32()),
        }) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                "cannot encode RegisterShard for topic {}: {}",
                topic.to_string(),
                error
            );
                return;
            }
        };

    if let Err(error) = broker.handle.send(packet) {
        tracing::error!("failed to send packet to broker: {error:#}");
        return;
    }


    info!(
        "registered shard with broker topic={}",
        topic.to_string()
    );
}

fn handle_broker_message(
    config: &ServerConfig,
    commands: &mut Commands,
    broker: &mut BrokerShardPeer,
    registry: &mut SharedEntityRegistry,
    allocator: &mut EntityIdAllocator,
    data: &[u8],
    velocities: &mut Query<&mut Velocity>,
    spawn_players: &mut MessageWriter<SpawnPlayerEntityEvent>,
    spawn_ghosts: &mut MessageWriter<SpawnGhostEntityEvent>,
    promote_ghosts: &mut MessageWriter<PromoteGhostEvent>,
) {
    let message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode utils message in shard: {error:#}");
            return;
        }
    };

    let my_shard_id = ShardId(config.shard_topic.get_id_as_u32());

    match message {
        NetworkMessage::ClientInput { client_id, input } => {
            apply_client_input(&registry, client_id, input, velocities);
        }
        NetworkMessage::RegisterClient { client_id, username } => {
            spawn_players.write(SpawnPlayerEntityEvent {
                client_id,
                username,
                position: Vec2::ZERO,
            });
            info!("queued player spawn for registered client_id={}", client_id.0);
        }
        NetworkMessage::UnregisterClient { client_id } => {
            match registry.try_lock() {
                Some((mut cli_registry, mut ent_registry)) => {
                    let Some(entity_id) = cli_registry.remove_client(&client_id) else {
                        tracing::warn!("could not find player for client_id={}", client_id.0);
                        return;
                    };
                    ent_registry.remove_by_entity_id(&entity_id);
                    if let Err(error) = broker.send_message_to_broker(&NetworkMessage::UnregisterEntity { entity_id }) {
                        tracing::error!("failed to send UnregisterEntity to broker: {error:#}");
                    }
                }
                None => tracing::warn!("could not lock registry for client unregistering"),
            }
        }
        NetworkMessage::EntityIdBlockAllocated { start, count } => {
            allocator.add_range(start, count);
        }
        // Ghost position sync from source shard — update ghost entity on this (dest) shard.
        NetworkMessage::GhostUpdate { entity_id, position, velocity } => {
            if let Some((_, ent_reg)) = registry.try_lock() {
                if let Some(bevy_entity) = ent_reg.get_bevy_entity(&entity_id) {
                    commands.entity(bevy_entity).insert((
                        crate::world::Position(Vec2::new(position.x as f32, position.y as f32)),
                        crate::world::Velocity(Vec2::new(velocity.x as f32, velocity.y as f32)),
                    ));
                }
            }
        }
        // Source shard: broker relays HandoffStart → send HandoffRequest to dest.
        NetworkMessage::HandoffStart { entity_id, source, destination } => {
            if source != my_shard_id {
                tracing::warn!(
                    "HandoffStart for entity {} has wrong source {} (my={})",
                    entity_id.0, source.0, my_shard_id.0
                );
                return;
            }
            handle_handoff_start_on_source(
                config, broker, registry, entity_id, destination,
                NetVec2::ZERO, NetVec2::ZERO,
            );
        }
        // Dest shard: broker relays HandoffRequest → spawn ghost + send HandoffAccepted.
        NetworkMessage::HandoffRequest { entity_id, position, velocity, .. } => {
            handle_handoff_request_on_dest(
                broker, spawn_ghosts, registry, entity_id,
                ShardId(0), // source implicit via broker routing
                position, velocity,
            );
        }
        // Source shard: HandoffAccepted from dest → enter ghost phase.
        NetworkMessage::HandoffAccepted { entity_id } => {
            handle_handoff_accepted_on_source(commands, registry, entity_id, ShardId(0));
        }
        // Source shard: HandoffRejected from dest → cancel.
        NetworkMessage::HandoffRejected { entity_id } => {
            handle_handoff_rejected_on_source(commands, registry, entity_id);
        }
        // Dest shard: HandoffCompleted from source → promote ghost.
        NetworkMessage::HandoffCompleted { entity_id } => {
            promote_ghosts.write(PromoteGhostEvent { entity_id });
        }

        other => {
            warn!("unexpected broker message received by shard: {:?}", other);
        }
    }
}
