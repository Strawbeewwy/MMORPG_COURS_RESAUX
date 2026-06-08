use crate::config::ServerConfig;

use crate::world::state::{EntityRegistry};
use bevy::prelude::*;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameNetworkEvent, GamePeer, GameStreamReliability,
};
use shared::protocol::{NetworkMessage, Topic, decode_message, encode_message, BrokerHandle, BrokerConnectionState, ClientId, NetVec2, WorldSnapshot, WorldUpdate};
use std::sync::{Arc};
use tokio::sync::{Mutex, MutexGuard};
use shared::protocol::utils::utils::BinaryEncode;
use crate::net::apply_client_input;
use crate::net::area_of_interest::{is_inside_area_of_interest, DEFAULT_AREA_OF_INTEREST_RADIUS};
use crate::net::handoff::handle_handoff_request;
use crate::world::{ClientEntityRegistry, Velocity};



/**
World interaction should always be done through the SharedPlayerRegistry resource.
All interaction on entities must be done with entity_reg_shared,
client_reg_shared is used to get an entity_id from the client_id,
then we use the entity_id on entity_reg_shared to interact with the entity.
**/
#[derive(Resource, Clone)]
pub struct SharedEntityRegistry {
    pub entity_reg_shared: Arc<Mutex<EntityRegistry>>,
    pub client_reg_shared: Arc<Mutex<ClientEntityRegistry>>,
}


/**
When accessing the SharedEntityRegistry, use the try_lock method to acquire both locks.
This ensures that both locks are acquired atomically, preventing partial lock acquisition and potential deadlocks.

Easy copy and paste:

        match shared_registry.try_lock() {
            Some((cli_registry, ent_registry))=> {
                // Do Something
            }
            None => {
                tracing::warn!("could not lock player registry for client input");
                return;
            }
        }


**/
impl SharedEntityRegistry {
    pub fn try_lock(&self) -> Option<(MutexGuard<ClientEntityRegistry>, MutexGuard<EntityRegistry>)> {
        let client_lock = self.client_reg_shared.try_lock().ok()?;
        let entity_lock = self.entity_reg_shared.try_lock().ok()?;
        Some((client_lock, entity_lock))
    }
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
    registry: Res<SharedEntityRegistry>,
    mut velocities: Query<&mut Velocity>,
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

        handle_broker_event(&config, &mut broker, &registry, event,&mut velocities);
    }
}

fn handle_broker_event(
    config: &ServerConfig,
    broker: &mut BrokerShardPeer,
    registry: &SharedEntityRegistry,
    event: GameNetworkEvent,
    velocities: &mut Query<&mut Velocity>,
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

            handle_broker_message(config, registry, &data, velocities);
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

    if let Topic::ShardInstance(shard_id) = config.shard_topic {
        let packet = match encode_message(&NetworkMessage::RegisterShard {
           shard_id,
        }) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
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

    info!(
        "registered shard with broker topic={}",
        &config.shard_topic.to_string()
    );
}

fn handle_broker_message(
    config: &ServerConfig,
    registry: &SharedEntityRegistry,
    data: &[u8],
    velocities: &mut Query<&mut Velocity>,
) {
    let message = match decode_message(data) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!("failed to decode utils message in shard: {error:#}");
            return;
        }
    };

    match message {

        NetworkMessage::ClientInput {
            client_id,
            input } => {
            apply_client_input(&registry, client_id, input,velocities);
        }
        NetworkMessage::RegisterClient {
            client_id,
            username} => {

        }

        other => {
            warn!("unexpected broker message received by shard: {:?}", other);
        }
    }
}
