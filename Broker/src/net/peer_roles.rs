use shared::game_sockets::GameConnection;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRole {
    Client,
    Shard,
    SpatialService,
}

#[derive(Default)]
pub struct PeerRoles {
    roles: HashMap<GameConnection, PeerRole>,
}

impl PeerRoles {
    pub fn remove(&mut self, connection: GameConnection) {
        self.roles.remove(&connection);
    }

    pub fn register_role(
        &mut self,
        connection: GameConnection,
        role: PeerRole,
        message_name: &str,
    ) -> bool {
        match self.roles.get(&connection).copied() {
            Some(current_role) if current_role == role => true,

            Some(current_role) => {
                tracing::warn!(
                    "rejected {} from connection {}: already registered as {:?}, cannot become {:?}",
                    message_name,
                    connection.connection_id,
                    current_role,
                    role
                );

                false
            }

            None => {
                self.roles.insert(connection, role);

                tracing::info!(
                    "connection {} registered as {:?} via {}",
                    connection.connection_id,
                    role,
                    message_name
                );

                true
            }
        }
    }

    pub fn ensure(
        &self,
        connection: GameConnection,
        expected_role: PeerRole,
        message_name: &str,
    ) -> bool {
        match self.roles.get(&connection).copied() {
            Some(current_role) if current_role == expected_role => true,

            Some(current_role) => {
                tracing::warn!(
                    "rejected {} from connection {}: role mismatch current={:?} expected={:?}",
                    message_name,
                    connection.connection_id,
                    current_role,
                    expected_role
                );

                false
            }

            None => {
                tracing::warn!(
                    "rejected {} from unregistered connection {}: expected role {:?}",
                    message_name,
                    connection.connection_id,
                    expected_role
                );

                false
            }
        }
    }
}