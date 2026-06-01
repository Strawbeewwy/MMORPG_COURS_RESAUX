use std::time::{Duration, Instant};
use game_sockets::{GameConnection, GamePeer, GameStream};

/// Tracks the lifecycle of the outbound utils connection.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BrokerConnectionState {
    /// Initial connect attempt in progress (no Connected event yet).
    Connecting,
    /// QUIC handshake done, waiting for the reliable stream to be created.
    Connected,
    /// Stream ready — messages can be sent.
    Ready,
    /// Connection was lost; reconnect should be attempted.
    Disconnected,
}




/// Outbound QUIC connection to the utils.
/// Used to send Subscribe / Unsubscribe messages.
pub struct BrokerHandle {
    pub peer: GamePeer,
    pub connection: Option<GameConnection>,
    /// Reliable stream used to send utils control messages.
    pub stream: Option<GameStream>,
    /// Explicit connection state — never ambiguous between "not yet" and "lost".
    pub state: BrokerConnectionState,
    /// Number of consecutive failed reconnect attempts (used for exponential backoff).
    pub reconnect_attempt: u32,
    /// Earliest wall-clock time at which the next reconnect attempt may be made.
    pub reconnect_after: Option<Instant>,
}

impl BrokerHandle {
    pub fn new(peer: GamePeer) -> Self {
        Self {
            peer,
            connection: None,
            stream: None,
            state: BrokerConnectionState::Connecting,
            reconnect_attempt: 0,
            reconnect_after: None,
        }
    }

    /// Construct with an explicit initial state (use `Disconnected` when the
    /// startup connection attempt fails so `reconnect_broker_if_needed` retries).
    pub fn with_state(peer: GamePeer, state: BrokerConnectionState) -> Self {
        let reconnect_after = if state == BrokerConnectionState::Disconnected {
            Some(Instant::now() + Duration::from_secs(1))
        } else {
            None
        };
        Self {
            peer,
            connection: None,
            stream: None,
            state,
            reconnect_attempt: if reconnect_after.is_some() { 1 } else { 0 },
            reconnect_after,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state == BrokerConnectionState::Ready
    }

    /// Send raw bytes over the reliable utils stream if ready.
    pub fn send(&self, payload: Vec<u8>) {
        let (Some(conn), Some(stream)) = (self.connection.as_ref(), self.stream.as_ref()) else {
            //tracing::warn!("BrokerClient not ready (state={:?}) — dropping message", self.state);
            return;
        };
        if let Err(e) = self.peer.send(conn, stream, payload.into()) {
            //tracing::error!("failed to send to utils: {e}");
        }
    }

    /// Reset state for a reconnect attempt (does not touch backoff counters).
    pub fn reset_for_reconnect(&mut self) {
        self.connection = None;
        self.stream = None;
        self.state = BrokerConnectionState::Connecting;
    }

    /// Reset backoff on successful connection establishment.
    pub fn reset_backoff(&mut self) {
        self.reconnect_attempt = 0;
        self.reconnect_after = None;
    }
}

