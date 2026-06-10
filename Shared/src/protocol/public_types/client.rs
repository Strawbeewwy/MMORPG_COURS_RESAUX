use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default , Hash, Serialize, Deserialize)]
pub struct ClientId(pub u32);

impl From<ClientId> for u32 {
    #[inline]
    fn from(client_id: ClientId) -> Self {
        client_id.0
    }
}

pub const CLIENT_ID_LEN: usize = size_of::<ClientId>();