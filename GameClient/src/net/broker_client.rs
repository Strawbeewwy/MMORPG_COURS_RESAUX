use crate::config::DEFAULT_RECONNECT_INTERVAL;
use bevy::prelude::*;
use bytes::Bytes;
use shared::game_sockets::{GameConnection, GamePeer, GameStream};
use shared::protocol::broker::Topic;
use std::collections::HashSet;
use std::time::Duration;

#[derive(Resource)]
pub struct BrokerClient {
    pub peer: Option<GamePeer>,
    pub connection: Option<GameConnection>,
    pub reliable_stream: Option<GameStream>,
    pub connected: bool,
    pub subscribed_topics: HashSet<Topic>,
    pub reconnect_timer: Timer,
}

impl Default for BrokerClient {
    fn default() -> Self {
        Self {
            peer: None,
            connection: None,
            reliable_stream: None,
            connected: false,
            subscribed_topics: HashSet::new(),
            reconnect_timer: Timer::new(
                Duration::from_secs(DEFAULT_RECONNECT_INTERVAL),
                TimerMode::Repeating,
            ),
        }
    }
}

impl BrokerClient {
    pub fn reset_connection(&mut self) {
        self.peer = None;
        self.connection = None;
        self.reliable_stream = None;
        self.connected = false;
        self.subscribed_topics.clear();
    }

    pub fn mark_disconnected(&mut self) {
        self.connection = None;
        self.reliable_stream = None;
        self.connected = false;
        self.subscribed_topics.clear();
    }

    pub fn is_ready(&self) -> bool {
        self.peer.is_some()
            && self.connection.is_some()
            && self.reliable_stream.is_some()
            && self.connected
    }

    pub fn send_raw(&self, payload: Vec<u8>) -> bool {
        let Some(peer) = self.peer.as_ref() else {
            tracing::warn!("cannot send broker packet: peer is not ready");
            return false;
        };

        let Some(connection) = self.connection else {
            tracing::warn!("cannot send broker packet: not connected to broker yet");
            return false;
        };

        let Some(stream) = self.reliable_stream.as_ref() else {
            tracing::warn!("cannot send broker packet: reliable stream is not ready yet");
            return false;
        };

        match peer.send(&connection, stream, Bytes::from(payload)) {
            Ok(()) => true,
            Err(error) => {
                tracing::error!("failed to send broker packet: {}", error);
                false
            }
        }
    }
}