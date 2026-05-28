
pub type ClientId = u32;
pub const CLIENT_ID_LEN: usize = size_of::<ClientId>();

pub const TOPIC_LEN: usize = 32;
pub type Topic = [u8; TOPIC_LEN];


use crate::protocol::broker::config::CLIENT_INPUT_LEN;

#[derive(Debug, Clone)]
pub enum BrokerMessage {
    Subscribe {
        client_id: ClientId,
        topic: Topic,
    },
    Unsubscribe {
        client_id: ClientId,
        topic: Topic,
    },
    Publish {
        topic: Topic,
        payload: Vec<u8>,
    },
    Broadcast {
        payload: Vec<u8>,
    },
    ClientInput {
        client_id: ClientId,
        input: [u8; CLIENT_INPUT_LEN],
    },
    RegisterClient {
        client_id: ClientId,
    },
    RegisterShard {
        topic: Topic,
    },
    RegisterSpatialService,
    AddClientToShard {
        topic: Topic,
        client_id: ClientId,
        payload: Vec<u8>,
    },
    SetClientAuthority {
        client_id: ClientId,
        topic: Topic,
    },
    ClientHello{
        username: String,
    },
    ClientAccepted {
        client_id: ClientId,
    },
    PositionUpdate {
        client_id: ClientId,
        position: [f32; 2],
    },
}