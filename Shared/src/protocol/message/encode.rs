use crate::protocol::message::config::*;
pub use crate::protocol::message::network_message::NetworkMessage;
pub use crate::protocol::public_types::topic::{
    ShardId,
    Topic,
    TOPIC_LEN,
};
pub use crate::protocol::game::entity::{
    EntityId,
    EntityState,
    EntityType,
    ENTITY_ID_LEN,
    ENTITY_STATE_LEN,
};
use crate::protocol::{
    ClientId,
    NetVec2,
    Username,
    CLIENT_ID_LEN,
};
use crate::protocol::public_types::topic::TOPIC_ID_LEN;
use crate::protocol::utils::utils::{
    BinaryEncode,
    write_client_id,
    write_net_vec2,
    write_u8,
    write_u16,
    write_u32,
    write_username,
};

pub fn encode_message(message: &NetworkMessage) -> anyhow::Result<Vec<u8>> {
    match message {
        NetworkMessage::Subscribe { client_id, shard_id } => {
            encode_subscribe(*client_id, Topic::ShardInstance(*shard_id))
        }
        NetworkMessage::Unsubscribe { client_id, shard_id } => {
            encode_unsubscribe(*client_id, Topic::ShardInstance(*shard_id))
        }
        NetworkMessage::Publish {
            shard_id,
            client_id,
            payload_len,
            payload,
        } => {
            encode_publish(
                Topic::ShardInstance(*shard_id),
                *client_id,
                *payload_len,
                payload,
            )
        }
        NetworkMessage::Broadcast { payload_len, payload } => {
            encode_broadcast(*payload_len, payload)
        }
        NetworkMessage::ClientInput { client_id, input } => {
            encode_client_input(*client_id, *input)
        }
        NetworkMessage::RegisterShard { shard_id } => {
            encode_register_shard(Topic::ShardInstance(*shard_id))
        }
        NetworkMessage::RegisterSpatialService => {
            encode_register_spatial_service()
        }
        NetworkMessage::ClientHello { username } => {
            encode_client_hello(username)
        }
        NetworkMessage::ClientAccepted { client_id } => {
            encode_client_accepted(*client_id)
        }
        NetworkMessage::PositionUpdate { client_id, position } => {
            encode_position_update(*client_id, *position)
        }
        NetworkMessage::HandoffRequest {
            entity_id,
            from_shard_id,
            to_shard_id,
            position,
            velocity,
            entity_state,
        } => {
            encode_handoff_request(
                *entity_id,
                *from_shard_id,
                *to_shard_id,
                *position,
                *velocity,
                *entity_state,
            )
        }
        NetworkMessage::HandoffAccepted {
            entity_id,
            accepting_shard_id,
        } => {
            encode_handoff_accepted(*entity_id, *accepting_shard_id)
        }
        NetworkMessage::HandoffRejected {
            entity_id,
            rejecting_shard_id,
        } => {
            encode_handoff_rejected(*entity_id, *rejecting_shard_id)
        }
        NetworkMessage::GhostUpdate {
            entity_id,
            to_shard_id,
            position,
            velocity,
        } => {
            encode_ghost_update(*entity_id, *to_shard_id, *position, *velocity)
        }
        NetworkMessage::HandoffCompleted { entity_id } => {
            encode_handoff_completed(*entity_id)
        }
        NetworkMessage::RegisterClient { client_id, username } => {
            encode_register_client(*client_id, username)
        }
    }
}

fn encode_client_hello(username: &Username) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + username.len()
    );

    write_u8(&mut packet, TAG_CLIENT_HELLO);
    write_username(&mut packet, username)?;

    Ok(packet)
}

fn encode_register_client(
    client_id: ClientId,
    username: &Username,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + username.len() + CLIENT_ID_LEN
    );

    write_u8(&mut packet, TAG_CLIENT_REGISTER);
    write_username(&mut packet, username)?;
    write_client_id(&mut packet, client_id);

    Ok(packet)
}

fn encode_client_accepted(client_id: ClientId) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    write_u8(&mut packet, TAG_CLIENT_ACCEPTED);
    write_client_id(&mut packet, client_id);

    Ok(packet)
}

fn encode_subscribe(
    client_id: ClientId,
    topic: Topic,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN
    );

    write_u8(&mut packet, TAG_SUBSCRIBE);
    write_client_id(&mut packet, client_id);
    topic.encode_binary(&mut packet)?;

    Ok(packet)
}

fn encode_unsubscribe(
    client_id: ClientId,
    topic: Topic,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN
    );

    write_u8(&mut packet, TAG_UNSUBSCRIBE);
    write_client_id(&mut packet, client_id);
    topic.encode_binary(&mut packet)?;

    Ok(packet)
}

fn encode_publish(
    topic: Topic,
    client_id: ClientId,
    payload_len: u16,
    payload: &[u8],
) -> anyhow::Result<Vec<u8>> {
    if payload.len() != payload_len as usize {
        anyhow::bail!(
            "Publish payload length mismatch: declared={}, actual={}",
            payload_len,
            payload.len()
        );
    }

    let mut packet = Vec::with_capacity(
        TAG_LEN + TOPIC_LEN + CLIENT_ID_LEN + size_of::<u16>() + payload.len()
    );

    write_u8(&mut packet, TAG_PUBLISH);
    topic.encode_binary(&mut packet)?;
    write_client_id(&mut packet, client_id);
    write_u16(&mut packet, payload_len);
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_broadcast(
    payload_len: u16,
    payload: &[u8],
) -> anyhow::Result<Vec<u8>> {
    if payload.len() != payload_len as usize {
        anyhow::bail!(
            "Broadcast payload length mismatch: declared={}, actual={}",
            payload_len,
            payload.len()
        );
    }

    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + payload.len()
    );

    write_u8(&mut packet, TAG_BROADCAST);
    write_u16(&mut packet, payload_len);
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_client_input(
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + CLIENT_INPUT_LEN
    );

    write_u8(&mut packet, TAG_CLIENT_INPUT);
    write_client_id(&mut packet, client_id);
    packet.extend_from_slice(&input);

    Ok(packet)
}

fn encode_register_shard(topic: Topic) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN);

    write_u8(&mut packet, TAG_REGISTER_SHARD);
    topic.encode_binary(&mut packet)?;

    Ok(packet)
}

fn encode_register_spatial_service() -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(TAG_LEN);

    write_u8(&mut packet, TAG_REGISTER_SPATIAL_SERVICE);

    Ok(packet)
}

fn encode_position_update(
    client_id: ClientId,
    position: NetVec2,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN + 10
    );

    write_u8(&mut packet, TAG_POSITION_UPDATE);
    write_client_id(&mut packet, client_id);
    write_net_vec2(&mut packet, position);

    Ok(packet)
}

fn encode_handoff_completed(entity_id: EntityId) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(TAG_LEN + ENTITY_ID_LEN);

    write_u8(&mut packet, TAG_HANDOFF_COMPLETE);
    write_u32(&mut packet, entity_id.0);

    Ok(packet)
}

fn encode_handoff_accepted(
    entity_id: EntityId,
    accepting_shard_id: ShardId,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + TOPIC_ID_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_ACCEPTED);
    write_u32(&mut packet, entity_id.0);
    write_u32(&mut packet, accepting_shard_id.0);

    Ok(packet)
}

fn encode_handoff_rejected(
    entity_id: EntityId,
    rejecting_shard_id: ShardId,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + TOPIC_ID_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_REJECTED);
    write_u32(&mut packet, entity_id.0);
    write_u32(&mut packet, rejecting_shard_id.0);

    Ok(packet)
}

fn encode_handoff_request(
    entity_id: EntityId,
    from_shard_id: ShardId,
    to_shard_id: ShardId,
    position: NetVec2,
    velocity: NetVec2,
    entity_state: EntityState,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + (2 * TOPIC_ID_LEN) + 20 + ENTITY_STATE_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_REQUEST);
    write_u32(&mut packet, entity_id.0);
    write_u32(&mut packet, from_shard_id.0);
    write_u32(&mut packet, to_shard_id.0);
    write_net_vec2(&mut packet, position);
    write_net_vec2(&mut packet, velocity);
    packet.extend_from_slice(&entity_state.to_le_bytes());

    Ok(packet)
}

fn encode_ghost_update(
    entity_id: EntityId,
    to_shard_id: ShardId,
    position: NetVec2,
    velocity: NetVec2,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + TOPIC_ID_LEN + 20
    );

    write_u8(&mut packet, TAG_GHOST_UPDATE);
    write_u32(&mut packet, entity_id.0);
    write_u32(&mut packet, to_shard_id.0);
    write_net_vec2(&mut packet, position);
    write_net_vec2(&mut packet, velocity);

    Ok(packet)
}