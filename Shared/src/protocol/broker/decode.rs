use std::sync::Arc;
use crate::protocol::broker::config::*;
pub use crate::protocol::broker::broker_message::{
    CLIENT_ID_LEN, ClientId,BrokerMessage,
};
pub use crate::protocol::broker::topic::{
    ShardId,Topic,TOPIC_LEN, read_topic,
};
pub use crate::protocol::game::entity::{
    ENTITY_ID_LEN, EntityId,EntityType,ENTITY_STATE_LEN,
    EntityState,
};
pub use crate::protocol::broker::utils::{
    read_u32_le, read_u16_le,read_client_id,
};
use crate::protocol::{NetVec2, Username};




pub fn decode_message(data: &[u8]) -> anyhow::Result<BrokerMessage> {
    let Some((&tag_bytes, body)) = data.split_first() else {
        anyhow::bail!("empty broker message");
    };
    let tag = u8::from_le(tag_bytes);

    match tag {
        TAG_SUBSCRIBE => decode_subscribe(body),
        TAG_UNSUBSCRIBE => decode_unsubscribe(body),
        TAG_PUBLISH => decode_publish(body),
        TAG_BROADCAST => decode_broadcast(body),
        TAG_CLIENT_INPUT => decode_client_input(body),
        TAG_REGISTER_SHARD => decode_register_shard(body),
        TAG_REGISTER_SPATIAL_SERVICE => decode_register_spatial_service(body),
        TAG_CLIENT_HELLO => decode_client_hello(body),
        TAG_CLIENT_ACCEPTED => decode_client_accepted(body),
        TAG_POSITION_UPDATE => decode_position_update(body),
        TAG_CLIENT_REGISTER => decode_register_client(body),
        TAG_HANDOFF_REQUEST => decode_handoff_request(body),
        TAG_HANDOFF_ACCEPTED => decode_handoff_accepted(body),
        TAG_HANDOFF_REJECTED => decode_handoff_rejected(body),
        TAG_HANDOFF_COMPLETE => decode_handoff_complete(body),
        TAG_GHOST_UPDATE => decode_ghost_update(body),
        unknown => anyhow::bail!("unknown broker message tag: 0x{unknown:02x}"),
    }
}

fn decode_ghost_update(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != ENTITY_ID_LEN + 20 {
        anyhow::bail!("invalid packet length: {}", body.len());
    }

    let entity_id = EntityId(read_u32_le(&body[0..ENTITY_ID_LEN]));

    let mut position_bytes = [0u8; 10];
    position_bytes.copy_from_slice(&body[ENTITY_ID_LEN..ENTITY_ID_LEN+10]);
    let position = NetVec2::try_from(position_bytes);

    let mut velocity_bytes = [0u8; 10];
    velocity_bytes.copy_from_slice(&body[ENTITY_ID_LEN + 10..ENTITY_ID_LEN+20]);
    let velocity = NetVec2::try_from(velocity_bytes);


    Ok(BrokerMessage::GhostUpdate { entity_id, position:position.unwrap(),velocity:velocity.unwrap() })
}

fn decode_handoff_rejected(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != ENTITY_ID_LEN {
        anyhow::bail!("invalid packet length: {}", body.len());
    }

    let entity_id = EntityId(read_u32_le(&body[0..ENTITY_ID_LEN]));

    Ok(BrokerMessage::HandoffRejected { entity_id })
}

fn decode_handoff_complete(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != ENTITY_ID_LEN {
        anyhow::bail!("invalid packet length: {}", body.len());
    }

    let entity_id = EntityId(read_u32_le(&body[0..ENTITY_ID_LEN]));

    Ok(BrokerMessage::HandoffCompleted { entity_id })
}

fn decode_handoff_accepted(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != ENTITY_ID_LEN {
        anyhow::bail!("invalid packet length: {}", body.len());
    }

    let entity_id = EntityId(read_u32_le(&body[0..ENTITY_ID_LEN]));

    Ok(BrokerMessage::HandoffAccepted { entity_id })
}

fn decode_handoff_request(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != ENTITY_ID_LEN + 21 {
        anyhow::bail!("invalid packet length: {}", body.len());
    }

    let entity_id = EntityId(read_u32_le(&body[0..ENTITY_ID_LEN]));

    let mut position_bytes = [0u8; 10];
    position_bytes.copy_from_slice(&body[ENTITY_ID_LEN..ENTITY_ID_LEN+10]);
    let position = NetVec2::try_from(position_bytes);

    let mut velocity_bytes = [0u8; 10];
    velocity_bytes.copy_from_slice(&body[ENTITY_ID_LEN + 10..ENTITY_ID_LEN+20]);
    let velocity = NetVec2::try_from(velocity_bytes);


    let mut entity_state_byte = [0u8;1];
    entity_state_byte.copy_from_slice(&body[ENTITY_ID_LEN + 20..ENTITY_ID_LEN + 21]);
    let entity_state = EntityState::from_le_bytes(entity_state_byte);

    Ok(BrokerMessage::HandoffRequest { entity_id, position:position.unwrap(),velocity:velocity.unwrap(), entity_state: entity_state.unwrap() })

}

fn decode_position_update(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + 2 * size_of::<f32>() {
        anyhow::bail!("invalid PositionUpdate length: {}", body.len());
    }

    let client_id = read_client_id(&body[0..CLIENT_ID_LEN]);
    let mut position_bytes = [0u8; 10];
    position_bytes.copy_from_slice(&body[CLIENT_ID_LEN..CLIENT_ID_LEN+10]);
    let position = NetVec2::try_from(position_bytes);

    Ok(BrokerMessage::PositionUpdate { client_id, position: position.unwrap() })
}

fn decode_register_shard(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() !=  TOPIC_LEN {
        anyhow::bail!("invalid RegisterShard length: {}", body.len());
    }

    let topic = read_topic(&body[0..TOPIC_LEN]);

    if let Topic::ShardInstance(shard_id) = topic {
        Ok(BrokerMessage::RegisterShard { shard_id })
    } else {
        anyhow::bail!("Topic received is not a valid Shard instance")
    }
}

fn decode_register_spatial_service(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if !body.is_empty() {
        anyhow::bail!(
            "invalid RegisterSpatialService length: {}",
            body.len()
        );
    }

    Ok(BrokerMessage::RegisterSpatialService)
}

fn decode_subscribe(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + TOPIC_LEN {
        anyhow::bail!("invalid Subscribe length: {}", body.len());
    }

    let client_id = read_client_id(&body[0..CLIENT_ID_LEN]);
    let topic = read_topic(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + TOPIC_LEN]);

    if let Topic::ShardInstance(shard_id) = topic {
        Ok(BrokerMessage::Subscribe {client_id, shard_id })
    } else {
        anyhow::bail!("Topic received is not a valid Shard instance")
    }
}

fn decode_unsubscribe(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() !=  CLIENT_ID_LEN + TOPIC_LEN {
        anyhow::bail!("invalid Unsubscribe length: {}", body.len());
    }

    let client_id = read_client_id(&body[0..CLIENT_ID_LEN]);
    let topic = read_topic(&body[0 + CLIENT_ID_LEN.. CLIENT_ID_LEN + TOPIC_LEN]);

    if let Topic::ShardInstance(shard_id) = topic {
        Ok(BrokerMessage::Unsubscribe { client_id, shard_id })
    } else {
        anyhow::bail!("Topic received is not a valid Shard instance")
    }
}

fn decode_publish(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() > MAX_PAYLOAD_LEN {
        anyhow::bail!("invalid Publish length: {}", body.len());
    }

    let topic = read_topic(&body[0..TOPIC_LEN]);

    let expected_len = read_u16_le(&body[TOPIC_LEN..TOPIC_LEN + size_of::<u16>()]) as usize;

    if body.len() - TOPIC_LEN - size_of::<u16>() != expected_len {
        anyhow::bail!(
            "invalid Broadcast payload length: declared={}, actual={}",
            expected_len,
            body.len().saturating_sub(MAX_PAYLOAD_LEN)
        );
    }
    let payload_len = expected_len as u16;
    let payload = body[TOPIC_LEN + size_of::<u16>()..].to_vec();

    if let Topic::ShardInstance(shard_id) = topic {
        Ok(BrokerMessage::Publish { shard_id, payload_len, payload })
    } else {
        anyhow::bail!("Topic received is not a valid Shard instance")
    }
}

fn decode_broadcast(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() > MAX_PAYLOAD_LEN {
        anyhow::bail!("Broadcast too long: {}", body.len());
    }

    let expected_len = read_u16_le(&body[0.. size_of::<u16>()]) as usize;

    if body.len() - size_of::<u16>() != expected_len {
        anyhow::bail!(
            "invalid Broadcast payload length: declared={}, actual={}",
            expected_len,
            body.len().saturating_sub(MAX_PAYLOAD_LEN)
        );
    }
    let payload_len = expected_len as u16;
    let payload = body[size_of::<u16>()..].to_vec();

    Ok(BrokerMessage::Broadcast {payload_len, payload })
}

fn decode_client_input(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + CLIENT_INPUT_LEN {
        anyhow::bail!("invalid ClientInput length: {}", body.len());
    }

    let client_id = ClientId(read_u32_le(&body[0..CLIENT_ID_LEN]));

    let mut input = [0_u8; CLIENT_INPUT_LEN];
    input.copy_from_slice(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + CLIENT_INPUT_LEN]);

    Ok(BrokerMessage::ClientInput { client_id, input })
}

fn decode_client_hello(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.is_empty() {
        anyhow::bail!("invalid hello client: empty packet");
    }

    let username_len = read_u16_le(&body[0..2]);

    let username_bytes = &body[2..2 + username_len as usize];

    let username: Username = Arc::from(String::from_utf8(username_bytes.to_vec())?);

    Ok(BrokerMessage::ClientHello {username})
}

fn decode_register_client(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.is_empty() {
        anyhow::bail!("invalid register client: empty packet");
    }

    let username_len = read_u16_le(&body[0..2]);

    let username_bytes = &body[2..2 + username_len as usize];

    let client_id = ClientId(read_u32_le(&body[2 + username_len as usize..2 + username_len as usize + CLIENT_ID_LEN]));

    let username: Username = Arc::from(String::from_utf8(username_bytes.to_vec())?);

    Ok(BrokerMessage::RegisterClient {client_id, username })
}

fn decode_client_accepted(
    body: &[u8]
) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN {
        anyhow::bail!("invalid ClientAccepted length: {}", body.len());
    }

    let client_id = ClientId(read_u32_le(&body[0..CLIENT_ID_LEN]));

    Ok(BrokerMessage::ClientAccepted { client_id })
}




