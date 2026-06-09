use game_sockets::{GameConnection, GamePeer, GameStream};


pub struct SpatialHandle {
    pub connection :Option<GameConnection>,
    pub stream : Option<GameStream>,
}

impl Default for SpatialHandle {
    fn default() -> Self {
        Self {
            connection: None,
            stream: None,
        }
    }
}

impl SpatialHandle {
    /// Register a spatial's identity once it sends a SpatialRegister message.
    pub fn register_spatial(&mut self, connection: GameConnection, stream: GameStream) {
        self.connection = Some(connection.clone());
        self.stream = Some(stream.clone());
    }

    /// Remove spatial's registration on disconnect.
    pub fn unregister_shard(&mut self, conn: GameConnection) {
        if self.connection.is_some() && self.stream.is_some() {
            self.connection = None;
            self.stream = None;
        }
    }

    /// Send a raw payload to the spatial server.
    /// Returns `true` if the spatial is connected and the send succeeded.
    pub fn send_to_spatial(&self, peer: &GamePeer, payload: Vec<u8>) -> anyhow::Result<bool> {
        let Some(conn) = self.connection else {
            anyhow::bail!("spatial: not connected")
        };
        let Some(ref stream) = self.stream else {
            anyhow::bail!("spatial has no stream yet")
        };
        
        match peer.send(&conn, &stream, payload.into()) {
            Ok(_) => Ok(true),
            Err(e) => {
                anyhow::bail!("failed to send to spatial: {e}");
            }
        }
    }
}