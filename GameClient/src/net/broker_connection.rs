use crate::config::ClientConfig;
use crate::net::broker_client::BrokerClient;
use crate::net::broker_message::decode_and_handle_broker_message;
use crate::world::state::LocalWorldState;
use bevy::prelude::*;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameNetworkEvent, GamePeer, GameStreamReliability,
};
use shared::protocol::broker::{
    BrokerMessage, encode_message
};


pub fn connect_to_broker(
    config: Res<ClientConfig>,
    mut broker_client: ResMut<BrokerClient>,
    mut world_state: ResMut<LocalWorldState>,
) {
    tracing::info!(
        "starting GameClient for username={} broker={} zone={}",
        config.username,
        config.broker_addr(),
        config.zone
    );

    world_state.player_id = None;
    world_state.zone = Some(config.zone.clone());

    try_connect_to_broker(&config, &mut broker_client);
}

fn try_connect_to_broker(
    config: &ClientConfig,
    broker_client: &mut BrokerClient,
) {
    tracing::info!("trying to connect to broker {}", config.broker_addr());

    let peer = GamePeer::new(QuicBackend::new());

    if let Err(error) = peer.connect(&config.broker_ip, config.broker_port) {
        tracing::error!(
            "failed to start connection to broker {}: {}",
            config.broker_addr(),
            error
        );
        return;
    }

    broker_client.reset_connection();
    broker_client.peer = Some(peer);

    tracing::info!("connection attempt started to broker {}", config.broker_addr());
}

pub fn retry_broker_connection_if_needed(
    time: Res<Time>,
    config: Res<ClientConfig>,
    mut broker_client: ResMut<BrokerClient>,
) {
    if broker_client.connected || broker_client.connection.is_some() {
        return;
    }

    broker_client.reconnect_timer.tick(time.delta());

    if !broker_client.reconnect_timer.just_finished() {
        return;
    }

    tracing::info!(
        "not connected to broker yet; retrying {}",
        config.broker_addr()
    );

    broker_client.reset_connection();

    try_connect_to_broker(&config, &mut broker_client);
}

pub fn poll_broker_events(
    config: Res<ClientConfig>,
    mut broker_client: ResMut<BrokerClient>,
    mut world_state: ResMut<LocalWorldState>,
) {
    loop {
        let event = {
            let Some(peer) = broker_client.peer.as_mut() else {
                return;
            };

            match peer.poll() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(error) => {
                    tracing::warn!(
                        "broker peer poll failed while connecting to {}: {}",
                        config.broker_addr(),
                        error
                    );

                    broker_client.reset_connection();

                    break;
                }
            }
        };

        handle_broker_event(&config, &mut broker_client, &mut world_state, event);
    }
}

fn handle_broker_event(
    config: &ClientConfig,
    broker_client: &mut BrokerClient,
    world_state: &mut LocalWorldState,
    event: GameNetworkEvent,
) {
    match event {
        GameNetworkEvent::Connected(connection) => {
            tracing::info!("connected to broker: {}", connection.connection_id);

            broker_client.connection = Some(connection);

            let Some(peer) = broker_client.peer.as_mut() else {
                return;
            };

            if let Err(error) = peer.create_stream(connection, GameStreamReliability::Reliable) {
                tracing::error!(
                    "failed to create reliable stream for broker connection {}: {}",
                    connection.connection_id,
                    error
                );
            }
        }

        GameNetworkEvent::Disconnected(connection) => {
            tracing::warn!("disconnected from broker: {}", connection.connection_id);
            broker_client.mark_disconnected();
        }

        GameNetworkEvent::StreamCreated(connection, stream) => {
            tracing::info!(
                "broker stream created: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            if stream.is_reliable() {
                broker_client.connection = Some(connection);
                broker_client.reliable_stream = Some(stream);
                broker_client.connected = true;

                tracing::info!(
                    "GameClient connected to broker={}; sending ClientHello",
                    config.broker_addr()
                );

                let packet = match encode_message(&BrokerMessage::ClientHello {
                    username: config.username.clone(),
                }) {
                    Ok(packet) => packet,
                    Err(error) => {
                        tracing::warn!(
                "cannot encode ClientHello for client {}: {}",
                config.username,
                error
            );
                        return;
                    }
                };

                broker_client.send_raw(packet);

                tracing::info!(
                    "sent ClientHello; waiting for broker-assigned client_id"
                );
            }
        }

        GameNetworkEvent::StreamClosed(connection, stream) => {
            tracing::info!(
                "broker stream closed: connection={} stream={}",
                connection.connection_id,
                stream.stream_id
            );

            broker_client.reliable_stream = None;
            broker_client.connected = false;
            broker_client.subscribed_topics.clear();
        }

        GameNetworkEvent::Message {
            connection,
            stream,
            data,
        } => {
            tracing::debug!(
                "broker message received: connection={} stream={} bytes={}",
                connection.connection_id,
                stream.stream_id,
                data.len()
            );

            decode_and_handle_broker_message(broker_client, world_state, &data);
        }

        GameNetworkEvent::Error { connection, inner } => {
            tracing::warn!(
                "broker socket error on connection {} while targeting {}: {}",
                connection.connection_id,
                config.broker_addr(),
                inner
            );

            broker_client.reset_connection();
        }
    }
}