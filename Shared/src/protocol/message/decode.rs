use anyhow::anyhow;
pub use crate::protocol::*;

pub fn decode_message(
    data: &[u8]
) -> anyhow::Result<NetworkMessage> {
    let mut input = data;

    let tag = read_u8(&mut input)?;

    let message = match tag {
        TAG_SUBSCRIBE => decode_subscribe(&mut input)?,
        TAG_UNSUBSCRIBE => decode_unsubscribe(&mut input)?,
        TAG_PUBLISH => decode_publish(&mut input)?,
        TAG_BROADCAST => decode_broadcast(&mut input)?,
        TAG_CLIENT_INPUT => decode_client_input(&mut input)?,
        TAG_REGISTER_SHARD => decode_register_shard(&mut input)?,
        TAG_UNREGISTER_SHARD => decode_unregister_shard(&mut input)?,
        TAG_REGISTER_SPATIAL_SERVICE => decode_register_spatial_service(&mut input)?,
        TAG_CLIENT_HELLO => decode_client_hello(&mut input)?,
        TAG_CLIENT_REGISTER => decode_register_client(&mut input)?,
        TAG_CLIENT_ACCEPTED => decode_client_accepted(&mut input)?,
        TAG_REQUEST_ENTITY_ID_BLOCK => decode_request_entity_id_block(&mut input)?,
        TAG_ENTITY_ID_BLOCK_ALLOCATED => decode_entity_id_block_allocated(&mut input)?,
        TAG_POSITION_UPDATE => decode_position_update(&mut input)?,
        TAG_REGISTER_ENTITY => decode_register_entity(&mut input)?,
        TAG_HANDOFF_REQUEST => decode_handoff_request(&mut input)?,
        TAG_HANDOFF_ACCEPTED => decode_handoff_accepted(&mut input)?,
        TAG_HANDOFF_REJECTED => decode_handoff_rejected(&mut input)?,
        TAG_HANDOFF_COMPLETE => decode_handoff_complete(&mut input)?,
        TAG_GHOST_UPDATE => decode_ghost_update(&mut input)?,
        TAG_HANDOFF_START => decode_handoff_start(&mut input)?,
        TAG_UNREGISTER_CLIENT => decode_unregister_client(&mut input)?,
        TAG_UNREGISTER_ENTITY => decode_unregister_entity(&mut input)?,
        unknown => anyhow::bail!("unknown broker message tag: 0x{unknown:02x}"),
    };

    if !input.is_empty() {
        anyhow::bail!("trailing bytes after NetworkMessage: {}", input.len());
    }

    Ok(message)
}

fn decode_subscribe(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let client_id = read_client_id(input)?;
    let topic = Topic::decode_binary(input)?;

    Ok(NetworkMessage::Subscribe {
        client_id,
        topic,
    })
}

fn decode_unsubscribe(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let client_id = read_client_id(input)?;
    let topic = Topic::decode_binary(input)?;

    Ok(NetworkMessage::Unsubscribe {
        client_id,
        topic,
    })
}

fn decode_publish(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let topic = Topic::decode_binary(input)?;
    let payload_len = read_u16(input)?;
    let payload = read_exact(input, payload_len as usize)?.to_vec();

    Ok(NetworkMessage::Publish {
        topic,
        payload_len,
        payload,
    })
}

fn decode_broadcast(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let payload_len = read_u16(input)?;
    let payload = read_exact(input, payload_len as usize)?.to_vec();

    Ok(NetworkMessage::Broadcast {
        payload_len,
        payload,
    })
}

fn decode_client_input(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let client_id = read_client_id(input)?;
    let input_bytes = read_exact(input, CLIENT_INPUT_LEN)?;

    let mut client_input = [0_u8; CLIENT_INPUT_LEN];
    client_input.copy_from_slice(input_bytes);

    Ok(NetworkMessage::ClientInput {
        client_id,
        input: client_input,
    })
}

fn decode_register_shard(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let topic = Topic::decode_binary(input)?;

    let Topic::ShardInstance{id:shard_id} = topic else {
        anyhow::bail!("RegisterShard topic is not a ShardInstance");
    };

    Ok(NetworkMessage::RegisterShard {
        shard_id,
    })
}

fn decode_unregister_shard(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let topic = Topic::decode_binary(input)?;

    let Topic::ShardInstance { id: shard_id } = topic else {
        anyhow::bail!("UnregisterShard topic is not a ShardInstance");
    };

    Ok(NetworkMessage::UnregisterShard {
        shard_id,
    })
}

fn decode_register_spatial_service(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    if !input.is_empty() {
        anyhow::bail!(
            "invalid RegisterSpatialService length: {}",
            input.len()
        );
    }

    Ok(NetworkMessage::RegisterSpatialService)
}

fn decode_client_hello(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let username = read_username(input)?;

    Ok(NetworkMessage::ClientHello {
        username,
    })
}

fn decode_register_client(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let username = read_username(input)?;
    let client_id = read_client_id(input)?;

    Ok(NetworkMessage::RegisterClient {
        client_id,
        username,
    })
}

fn decode_client_accepted(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let client_id = read_client_id(input)?;

    Ok(NetworkMessage::ClientAccepted {
        client_id,
    })
}

fn decode_request_entity_id_block(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let shard_id = ShardId(read_u32(input)?);
    let count = read_u32(input)?;

    Ok(NetworkMessage::RequestEntityIdBlock {
        shard_id,
        count,
    })
}

fn decode_entity_id_block_allocated(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let start = read_u32(input)?;
    let count = read_u32(input)?;

    Ok(NetworkMessage::EntityIdBlockAllocated {
        start,
        count,
    })
}

fn decode_position_update(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);
    let position = read_net_vec2(input)?;

    Ok(NetworkMessage::PositionUpdate {
        entity_id,
        position,
    })
}

fn decode_register_entity(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);
    let client_id = ClientId(read_u32(input)?);
    let position = read_net_vec2(input)?;

    Ok(NetworkMessage::RegisterEntity {
        entity_id,
        client_id,
        position,
    })
}

fn decode_handoff_complete(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);

    Ok(NetworkMessage::HandoffCompleted {
        entity_id,
    })
}

fn decode_handoff_accepted(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);

    Ok(NetworkMessage::HandoffAccepted {
        entity_id,
    })
}

fn decode_handoff_rejected(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);

    Ok(NetworkMessage::HandoffRejected {
        entity_id,
    })
}

fn decode_handoff_request(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);
    let position = read_net_vec2(input)?;
    let velocity = read_net_vec2(input)?;

    let entity_state_bytes = read_exact(input, 1)?;
    let mut entity_state_byte = [0_u8; 1];
    entity_state_byte.copy_from_slice(entity_state_bytes);

    let entity_state = EntityState::from_le_bytes(entity_state_byte)
        .ok_or_else(|| anyhow!("Invalid EntityState byte: {}", entity_state_byte[0]))?;

    Ok(NetworkMessage::HandoffRequest {
        entity_id,
        position,
        velocity,
        entity_state,
    })
}

fn decode_ghost_update(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);
    let position = read_net_vec2(input)?;
    let velocity = read_net_vec2(input)?;

    Ok(NetworkMessage::GhostUpdate {
        entity_id,
        position,
        velocity,
    })
}

fn decode_handoff_start(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage>{
    let entity_id = EntityId(read_u32(input)?);
    let from = ShardId(read_u32(input)?);
    let to = ShardId(read_u32(input)?);

    Ok(NetworkMessage::HandoffStart {
        entity_id,
        source: from,
        destination: to
    })
}

fn decode_unregister_client(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let client_id = ClientId(read_u32(input)?);

    Ok(NetworkMessage::UnregisterClient {
        client_id,
    })
}

fn decode_unregister_entity(
    input: &mut &[u8]
) -> anyhow::Result<NetworkMessage> {
    let entity_id = EntityId(read_u32(input)?);

    Ok(NetworkMessage::UnregisterEntity {
        entity_id,
    })
}
