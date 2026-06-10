pub use crate::protocol::*;

pub fn encode_message(
    message: &NetworkMessage
) -> anyhow::Result<Vec<u8>> {
    match message {
        NetworkMessage::Subscribe { client_id, topic } => {
            encode_subscribe(*client_id, *topic)
        }
        NetworkMessage::Unsubscribe { client_id, topic } => {
            encode_unsubscribe(*client_id, *topic)
        }
        NetworkMessage::Publish { topic, payload_len, payload, } => {
            encode_publish(*topic, *payload_len, payload, )
        }
        NetworkMessage::Broadcast { payload_len, payload } => {
            encode_broadcast(*payload_len, payload)
        }
        NetworkMessage::ClientInput { client_id, input } => {
            encode_client_input(*client_id, *input)
        }
        NetworkMessage::RegisterShard { shard_id } => {
            encode_register_shard(Topic::ShardInstance{id:*shard_id})
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
        NetworkMessage::RequestEntityIdBlock { shard_id, count } => {
            encode_request_entity_id_block(*shard_id, *count)
        }
        NetworkMessage::EntityIdBlockAllocated { start, count, } => {
            encode_entity_id_block_allocated(*start, *count)
        }
        NetworkMessage::PositionUpdate { entity_id, position } => {
            encode_position_update(*entity_id, *position)
        }
        NetworkMessage::HandoffRequest { entity_id, position, velocity, entity_state, } => {
            encode_handoff_request(*entity_id, *position, *velocity, *entity_state, )
        }
        NetworkMessage::HandoffAccepted { entity_id, } => {
            encode_handoff_accepted(*entity_id)
        }
        NetworkMessage::HandoffRejected { entity_id, } => {
            encode_handoff_rejected(*entity_id)
        }
        NetworkMessage::GhostUpdate { entity_id, position, velocity, } => {
            encode_ghost_update(*entity_id, *position, *velocity, )
        }
        NetworkMessage::HandoffCompleted { entity_id, } => {
            encode_handoff_completed(*entity_id)
        }
        NetworkMessage::RegisterClient { client_id, username } => {
            encode_register_client(*client_id, username)
        }
        NetworkMessage::HandoffStart { entity_id, source, destination } => {
            encode_handoff_start(*entity_id,*source,*destination)
        }
        NetworkMessage::RegisterEntity { entity_id,position } => {
            encode_register_entity(*entity_id, *position)
        }
        NetworkMessage::UnregisterClient { client_id } => {
            encode_unregister_client(*client_id)
        }
        NetworkMessage::UnregisterEntity { entity_id } => {
            encode_unregister_entity(*entity_id)
        }
    }
}


fn encode_client_hello(
    username: &Username
) -> anyhow::Result<Vec<u8>> {
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

fn encode_client_accepted(
    client_id: ClientId
) -> anyhow::Result<Vec<u8>> {
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
        TAG_LEN + TOPIC_LEN  + size_of::<u16>() + payload.len()
    );

    write_u8(&mut packet, TAG_PUBLISH);
    topic.encode_binary(&mut packet)?;
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

fn encode_register_shard(
    topic: Topic
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN);

    write_u8(&mut packet, TAG_REGISTER_SHARD);
    topic.encode_binary(&mut packet)?;

    Ok(packet)
}

fn encode_register_spatial_service()
    -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(TAG_LEN);

    write_u8(&mut packet, TAG_REGISTER_SPATIAL_SERVICE);

    Ok(packet)
}
fn encode_request_entity_id_block(
    shard_id: ShardId,
    count: u32,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + TOPIC_ID_LEN + size_of::<u32>()
    );

    write_u8(&mut packet, TAG_REQUEST_ENTITY_ID_BLOCK);
    write_u32(&mut packet, shard_id.0);
    write_u32(&mut packet, count);

    Ok(packet)
}

fn encode_entity_id_block_allocated(
    start: u32,
    count: u32,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u32>() + size_of::<u32>()
    );

    write_u8(&mut packet, TAG_ENTITY_ID_BLOCK_ALLOCATED);
    write_u32(&mut packet, start);
    write_u32(&mut packet, count);

    Ok(packet)
}

fn encode_position_update(
    entity_id: EntityId,
    position: NetVec2,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + 10
    );

    write_u8(&mut packet, TAG_POSITION_UPDATE);
    write_u32(&mut packet, entity_id.0);
    write_net_vec2(&mut packet, position);

    Ok(packet)
}

fn encode_handoff_completed(
    entity_id: EntityId,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_COMPLETE);
    write_u32(&mut packet, entity_id.0);

    Ok(packet)
}

fn encode_handoff_accepted(
    entity_id: EntityId,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_ACCEPTED);
    write_u32(&mut packet, entity_id.0);

    Ok(packet)
}

fn encode_handoff_rejected(
    entity_id: EntityId,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_REJECTED);
    write_u32(&mut packet, entity_id.0);

    Ok(packet)
}

fn encode_handoff_request(
    entity_id: EntityId,
    position: NetVec2,
    velocity: NetVec2,
    entity_state: EntityState,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN
            + ENTITY_ID_LEN
            + 20
            + ENTITY_STATE_LEN
    );

    write_u8(&mut packet, TAG_HANDOFF_REQUEST);
    write_u32(&mut packet, entity_id.0);
    write_net_vec2(&mut packet, position);
    write_net_vec2(&mut packet, velocity);
    packet.extend_from_slice(&entity_state.to_le_bytes());

    Ok(packet)
}

fn encode_ghost_update(
    entity_id: EntityId,
    position: NetVec2,
    velocity: NetVec2,
) -> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + 20
    );

    write_u8(&mut packet, TAG_GHOST_UPDATE);
    write_u32(&mut packet, entity_id.0);
    write_net_vec2(&mut packet, position);
    write_net_vec2(&mut packet, velocity);

    Ok(packet)
}

fn encode_handoff_start(
    entity_id: EntityId,
    source: ShardId,
    destination: ShardId
)-> anyhow::Result<Vec<u8>> {
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + (2*TOPIC_ID_LEN)
    );

    write_u8(&mut packet, TAG_HANDOFF_START);
    write_u32(&mut packet, entity_id.0);
    write_u32(&mut packet, source.0);
    write_u32(&mut packet, destination.0);

    Ok(packet)
}

fn encode_register_entity(
    entity_id: EntityId,
    position: NetVec2
)->anyhow::Result<Vec<u8>> {

    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN + 10
    );

    write_u8(&mut packet, TAG_REGISTER_ENTITY);
    write_u32(&mut packet, entity_id.0);
    write_net_vec2(&mut packet, position);

    Ok(packet)
}

fn encode_unregister_client(
    client_id: ClientId,
)->anyhow::Result<Vec<u8>>{
    let mut packet = Vec::with_capacity(
        TAG_LEN + CLIENT_ID_LEN
    );

    write_u8(&mut packet, TAG_UNREGISTER_CLIENT);
    write_u32(&mut packet, client_id.0);

    Ok(packet)
}

fn encode_unregister_entity(
    entity_id: EntityId
)->anyhow::Result<Vec<u8>>{
    let mut packet = Vec::with_capacity(
        TAG_LEN + ENTITY_ID_LEN
    );

    write_u8(&mut packet, TAG_UNREGISTER_ENTITY);
    write_u32(&mut packet, entity_id.0);

    Ok(packet)
}