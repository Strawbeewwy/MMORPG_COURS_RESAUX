use crate::net::message_handler::handle_message;
use crate::net::peer_roles::{PeerRoles};
use crate::pubsub::state::PubSubState;
use shared::game_sockets::protocols::QuicBackend;
use shared::game_sockets::{
    GameConnection, GameNetworkEvent, GamePeer, GameStream, GameStreamReliability,
};
use std::collections::HashMap;

pub struct BrokerNetwork {
    peer: GamePeer,
    reliable_streams: HashMap<GameConnection, GameStream>,
    peer_roles: PeerRoles,
}

impl BrokerNetwork {
    pub fn listen(port: u16) -> anyhow::Result<Self> {
        let peer = GamePeer::new(QuicBackend::new());

        peer.listen("0.0.0.0", port)?;

        tracing::info!("utils listening on 0.0.0.0:{port}");

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
                tracing::info!("peer connected to utils: {}", connection.connection_id);

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

                let stream  = self.reliable_streams.remove(&connection);
                let peer_role = self.peer_roles.remove(connection);
                state.remove_connection(peer_role.unwrap(),connection, stream.unwrap());
            }

            GameNetworkEvent::StreamCreated(connection, stream) => {
                tracing::info!(
                    "utils stream created: connection={} stream={}",
                    connection.connection_id,
                    stream.stream_id
                );

                if stream.is_reliable() {
                    self.reliable_streams.insert(connection, stream);
                }
            }

            GameNetworkEvent::StreamClosed(connection, stream) => {
                tracing::info!(
                    "utils stream closed: connection={} stream={}",
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
                    state,
                    connection,
                    stream,
                    &data,
                );
            }

            GameNetworkEvent::Error { connection, inner } => {
                tracing::warn!(
                    "utils socket error on connection {}: {}",
                    connection.connection_id,
                    inner
                );
            }
        }
    }
}