use crate::protocol::message::config::*;
pub use crate::protocol::message::network_message::{
    NetworkMessage,
};
pub use crate::protocol::public_types::topic::{
    TOPIC_LEN, ShardId,Topic,
};
pub use crate::protocol::game::entity::{
    ENTITY_ID_LEN, EntityId,EntityType,EntityState,
    ENTITY_STATE_LEN,
};
use crate::protocol::{NetVec2, Username, ClientId,CLIENT_ID_LEN};


pub fn encode_message(message: &NetworkMessage) -> anyhow::Result<Vec<u8>> {
    match message {
        NetworkMessage::Subscribe { client_id, shard_id } => {
            Ok(encode_subscribe(*client_id, Topic::ShardInstance(*shard_id)))
        }
        NetworkMessage::Unsubscribe { client_id, shard_id } => {
            Ok(encode_unsubscribe(*client_id, Topic::ShardInstance(*shard_id)))
        }
        NetworkMessage::Publish { shard_id,payload_len, payload } => {
            encode_publish(Topic::ShardInstance(*shard_id),*payload_len, payload)
        }
        NetworkMessage::Broadcast { payload_len, payload } => {
            encode_broadcast(*payload_len,payload)
        }
        NetworkMessage::ClientInput { client_id, input } => {
            Ok(encode_client_input(*client_id, *input))
        }
        NetworkMessage::RegisterShard { shard_id } => {
            Ok(encode_register_shard(Topic::ShardInstance(*shard_id)))
        }
        NetworkMessage::RegisterSpatialService => {
            Ok(encode_register_spatial_service())
        }
        NetworkMessage::ClientHello {username} => {
            Ok(encode_client_hello(username.clone()))
        }
        NetworkMessage::ClientAccepted { client_id } => {
            Ok(encode_client_accepted(*client_id))
        },
        NetworkMessage::PositionUpdate { client_id, position, } => {
            Ok(encode_position_update(*client_id, *position))
        },
        NetworkMessage::HandoffRequest { entity_id, position, velocity, entity_state } => {
            Ok(encode_handoff_request(*entity_id,*position,*velocity,*entity_state))
        }
        NetworkMessage::HandoffAccepted {entity_id } => {
            Ok(encode_handoff_accepted(*entity_id))
        }
        NetworkMessage::HandoffRejected { entity_id } => {
            Ok(encode_handoff_rejected(*entity_id))
        }
        NetworkMessage::GhostUpdate { entity_id,position,velocity } => {
            Ok(encode_ghost_update(*entity_id,*position,*velocity))
        }
        NetworkMessage::HandoffCompleted {entity_id } => {
            Ok(encode_handoff_completed(*entity_id))
        }
        NetworkMessage::RegisterClient { client_id, username } => {
            Ok(encode_register_client(*client_id, username.clone()))
        }
    }
}

fn encode_client_hello(username: Username) -> Vec<u8> {
    let tag: u8 = TAG_CLIENT_HELLO;
    let username_bytes = username.as_bytes();

    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + username_bytes.len()
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&(username_bytes.len() as u16).to_le_bytes());
    packet.extend_from_slice(username_bytes);

    packet
}

fn encode_handoff_completed(
    entity_id: EntityId
) -> Vec<u8> {
    let tag: u8 = TAG_HANDOFF_COMPLETE;

    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&entity_id.0.to_le_bytes());

    packet
}

fn encode_ghost_update(
    entity_id: EntityId,
    position: NetVec2,
    velocity: NetVec2
) -> Vec<u8> {
    let tag: u8 = TAG_GHOST_UPDATE;

    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + 20// 10 per NetVec2
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&entity_id.0.to_le_bytes());
    packet.extend_from_slice(&position.to_bytes());
    packet.extend_from_slice(&velocity.to_bytes());

    packet
}

fn encode_handoff_rejected(
    entity_id: EntityId
) -> Vec<u8> {
    let tag: u8 = TAG_HANDOFF_REJECTED;

    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&entity_id.0.to_le_bytes());

    packet
}

fn encode_handoff_accepted(
    entity_id: EntityId
) -> Vec<u8> {
    let tag: u8 = TAG_HANDOFF_ACCEPTED;

    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&entity_id.0.to_le_bytes());

    packet
}

fn encode_handoff_request(
    entity_id: EntityId,
    position: NetVec2,
    velocity: NetVec2,
    entity_state: EntityState
) -> Vec<u8> {
    let tag: u8 = TAG_HANDOFF_REQUEST;

    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + ENTITY_STATE_LEN + 20// 10 per NetVec2
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&entity_id.0.to_le_bytes());
    packet.extend_from_slice(&position.to_bytes());
    packet.extend_from_slice(&velocity.to_bytes());
    packet.extend_from_slice(&entity_state.to_le_bytes());

    packet
}

fn encode_position_update(
    client_id: ClientId,
    positions: NetVec2
) -> Vec<u8> {
    let tag= TAG_POSITION_UPDATE;
    let id: u32 = client_id.into();

    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + 10//10 because NetVec2 is 10 bytes
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&positions.to_bytes());

    packet
}

fn encode_register_shard(
    topic: Topic
) -> Vec<u8> {
    let tag = TAG_REGISTER_SHARD;

    let mut packet = Vec::with_capacity(
        TAG_LEN + TOPIC_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}

fn encode_register_spatial_service(

) -> Vec<u8> {
    let tag: u8 = TAG_REGISTER_SPATIAL_SERVICE;

    let mut packet = Vec::with_capacity(
        TAG_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());

    packet
}

fn encode_subscribe(
    client_id: ClientId,
    topic: Topic)
    -> Vec<u8> {
    let tag: u8 = TAG_SUBSCRIBE;
    let id: u32 = client_id.into();
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}


fn encode_unsubscribe(
    client_id: ClientId,
    topic: Topic
) -> Vec<u8> {
    let tag: u8 = TAG_UNSUBSCRIBE;
    let id: u32 = client_id.into();
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());

    packet
}

fn encode_publish(
    topic: Topic,
    payload_len: u16,
    payload: &[u8]
) -> anyhow::Result<Vec<u8>> {
    let tag: u8 = TAG_PUBLISH;

    let mut packet = Vec::with_capacity(
        TAG_LEN + TOPIC_LEN + MAX_PAYLOAD_LEN + payload.len()
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&topic.to_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_broadcast(
    payload_len:u16,
    payload: &[u8]
) -> anyhow::Result<Vec<u8>> {
    let tag: u8 = TAG_BROADCAST;

    let mut packet = Vec::with_capacity(
        TAG_LEN + MAX_PAYLOAD_LEN + payload.len()
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_client_input(
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) -> Vec<u8> {
    let tag: u8 = TAG_CLIENT_INPUT;
    let id: u32 = client_id.into();
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + CLIENT_INPUT_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&input);

    packet
}
fn encode_register_client(
    client_id: ClientId,
    username: Username,
) -> Vec<u8> {
    let tag: u8 = TAG_CLIENT_HELLO;
    let username_bytes = username.as_bytes();

    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + username_bytes.len() + CLIENT_ID_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&(username_bytes.len() as u16).to_le_bytes());
    packet.extend_from_slice(username_bytes);
    packet.extend_from_slice(&client_id.0.to_le_bytes());

    packet
}

fn encode_client_accepted(
    client_id: ClientId
) -> Vec<u8> {
    let tag: u8 = TAG_CLIENT_ACCEPTED;

    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN
    );
    packet.extend_from_slice(&tag.to_le_bytes());
    packet.extend_from_slice(&client_id.0.to_le_bytes());

    packet
}


