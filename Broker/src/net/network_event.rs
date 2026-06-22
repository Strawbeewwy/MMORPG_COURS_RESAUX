use crate::net::message_handler::handle_message;
use crate::net::peer_roles::{PeerRole, PeerRoles};
use crate::pubsub::state::{ConnectionStream, PubSubState};
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use std::collections::HashMap;
use shared::{encode_message, NetworkMessage, Topic};
use crate::net::relay::relay_to_spatial_services;

pub struct BrokerNetwork {
    peer: GamePeer,
    reliable_streams: HashMap<GameConnection, GameStream>,
    peer_roles: PeerRoles,
}

impl BrokerNetwork {
    pub fn listen(port: u16) -> anyhow::Result<Self> {
        let peer = GamePeer::new(QuicBackend::new());

        peer.listen("0.0.0.0", port)?;

        tracing::info!("broker listening on 0.0.0.0:{port}");

        Ok(Self {
            peer,
            reliable_streams: HashMap::new(),
            peer_roles: PeerRoles::default(),
        })
    }

    pub fn poll_events(&mut self, state: &mut PubSubState) {
        loop {
            let event = match self.peer.poll() {
                Ok(Some(event)) => event,
                Ok(None) => break,
                Err(error) => {
                    tracing::error!("failed to poll utils peer: {error}");
                    break;
                }
            };

            self.handle_event(state, event);
        }
    }

    fn handle_event(
        &mut self,
        state: &mut PubSubState,
        event: GameNetworkEvent,
    ) {
        match event {
            GameNetworkEvent::Connected(connection) => {
                tracing::info!("peer connected to broker: {}", connection.connection_id);

                if let Err(error) = self
                    .peer
                    .create_stream(connection, GameStreamReliability::Reliable)
                {
                    tracing::error!(
                        "failed to create reliable stream for connection {}: {}",
                        connection.connection_id,
                        error
                    );
                }
            }

            GameNetworkEvent::Disconnected(connection) => {
                tracing::info!("peer disconnected from utils: {}", connection.connection_id);

                let stream = self.reliable_streams.remove(&connection);
                let peer_role = self.peer_roles.remove(connection);

                if let (Some(PeerRole::Shard), Some(stream)) = (peer_role, stream.clone()) {
                    let connection_stream = ConnectionStream {
                        connection,
                        stream: stream.clone(),
                    };

                    if let Some(topic) = state.get_shard_by_connection_stream(&connection_stream).copied() {
                        if let Topic::ShardInstance { id: shard_id } = topic {
                            if let Ok(packet) =
                                encode_message(&NetworkMessage::UnregisterShard { shard_id })
                            {
                                relay_to_spatial_services(&self.peer, state, &packet);
                            }
                        }
                    }
                }

                match (peer_role, stream) {
                    (Some(role), Some(s)) => state.remove_connection(role, connection, s),
                    (None, _) => {
                        tracing::warn!(
                                "disconnected connection {} had no registered role",
                                connection.connection_id
                            );
                    }
                    (Some(role), None) => {
                        tracing::warn!(
                                "disconnected connection {} role {:?} had no reliable stream",
                                connection.connection_id,
                                role
                            );
                    }
                }
            }

            GameNetworkEvent::StreamCreated(connection, stream) => {
                tracing::info!(
                    "broker stream created: connection={} stream={}",
                    connection.connection_id,
                    stream.stream_id
                );

                if stream.is_reliable() {
                    self.reliable_streams.insert(connection, stream);
                }
            }

            GameNetworkEvent::StreamClosed(connection, stream) => {
                tracing::info!(
                    "broker stream closed: connection={} stream={}",
                    connection.connection_id,
                    stream.stream_id
                );

                self.reliable_streams.remove(&connection);
            }

            GameNetworkEvent::Message {
                connection,
                stream,
                data,
            } => {
                handle_message(
                    &self.peer,
                    &mut self.peer_roles,
                    &self.reliable_streams,
                    state,
                    connection,
                    stream,
                    &data,
                );
            }

            GameNetworkEvent::Error { connection, inner } => {
                tracing::warn!(
                    "broker socket error on connection {}: {}",
                    connection.connection_id,
                    inner
                );
            }
        }
    }
}