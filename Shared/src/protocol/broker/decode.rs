
use crate::protocol::broker::config::*;

pub use crate::protocol::broker::broker_message::{
    CLIENT_ID_LEN, TOPIC_LEN, ClientId, Topic,BrokerMessage,
};

pub use crate::protocol::broker::utils::{
    topic_for_shard, topic_from_str, topic_to_string, read_u32_le,
    read_topic, read_u16_le,
};


pub fn decode_message(data: &[u8]) -> anyhow::Result<BrokerMessage> {
    let Some((&tag, body)) = data.split_first() else {
        anyhow::bail!("empty broker message");
    };

    match tag {
        TAG_SUBSCRIBE => decode_subscribe(body),
        TAG_UNSUBSCRIBE => decode_unsubscribe(body),
        TAG_PUBLISH => decode_publish(body),
        TAG_BROADCAST => decode_broadcast(body),
        TAG_CLIENT_INPUT => decode_client_input(body),
        TAG_REGISTER_CLIENT => decode_register_client(body),
        TAG_REGISTER_SHARD => decode_register_shard(body),
        TAG_REGISTER_SPATIAL_SERVICE => decode_register_spatial_service(body),
        TAG_ADD_CLIENT_TO_SHARD => decode_add_client_to_shard(body),
        TAG_SET_CLIENT_AUTHORITY => decode_set_client_authority(body),
        TAG_CLIENT_HELLO => decode_client_hello(body),
        TAG_CLIENT_ACCEPTED => decode_client_accepted(body),
        TAG_POSITION_UPDATE => decode_position_update(body),
        unknown => anyhow::bail!("unknown broker message tag: 0x{unknown:02x}"),
    }
}

fn decode_position_update(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != TAG_LEN + 2 * size_of::<f32>() {
        anyhow::bail!("invalid PositionUpdate length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);

    let x_start = CLIENT_ID_LEN;
    let x_end = x_start + size_of::<f32>();
    let y_start = x_end;
    let y_end = y_start + size_of::<f32>();

    let position = [
        f32::from_be_bytes(body[x_start..x_end].try_into()?),
        f32::from_be_bytes(body[y_start..y_end].try_into()?),
    ];
    Ok(BrokerMessage::PositionUpdate { client_id, position })
}

fn decode_register_client(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN {
        anyhow::bail!("invalid RegisterClient length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);

    Ok(BrokerMessage::RegisterClient { client_id })
}

fn decode_register_shard(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != TOPIC_LEN {
        anyhow::bail!("invalid RegisterShard length: {}", body.len());
    }

    let topic = read_topic(&body[0..TOPIC_LEN]);

    Ok(BrokerMessage::RegisterShard { topic })
}

fn decode_register_spatial_service(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if !body.is_empty() {
        anyhow::bail!(
            "invalid RegisterSpatialService length: {}",
            body.len()
        );
    }

    Ok(BrokerMessage::RegisterSpatialService)
}

fn decode_subscribe(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + TOPIC_LEN {
        anyhow::bail!("invalid Subscribe length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);
    let topic = read_topic(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + TOPIC_LEN]);

    Ok(BrokerMessage::Subscribe { client_id, topic })
}

fn decode_unsubscribe(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + TOPIC_LEN {
        anyhow::bail!("invalid Unsubscribe length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);
    let topic = read_topic(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + TOPIC_LEN]);

    Ok(BrokerMessage::Unsubscribe { client_id, topic })
}

fn decode_publish(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() < TOPIC_LEN + MAX_PAYLOAD_LEN_IN_BYTE {
        anyhow::bail!("Publish too short: {}", body.len());
    }

    let topic = read_topic(&body[0..TOPIC_LEN]);
    let payload_len_start = TOPIC_LEN;
    let payload_len_end = TOPIC_LEN + MAX_PAYLOAD_LEN_IN_BYTE;
    let payload_len = read_u16_le(&body[payload_len_start..payload_len_end]) as usize;

    let expected_len = TOPIC_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload_len;

    if body.len() != expected_len {
        anyhow::bail!(
            "invalid Publish payload length: declared={}, actual={}",
            payload_len,
            body.len().saturating_sub(TOPIC_LEN + MAX_PAYLOAD_LEN_IN_BYTE)
        );
    }

    let payload = body[payload_len_end..].to_vec();

    Ok(BrokerMessage::Publish { topic, payload })
}

fn decode_broadcast(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() < MAX_PAYLOAD_LEN_IN_BYTE {
        anyhow::bail!("Broadcast too short: {}", body.len());
    }

    let payload_len = read_u16_le(&body[0..MAX_PAYLOAD_LEN_IN_BYTE]) as usize;
    let expected_len = MAX_PAYLOAD_LEN_IN_BYTE + payload_len;

    if body.len() != expected_len {
        anyhow::bail!(
            "invalid Broadcast payload length: declared={}, actual={}",
            payload_len,
            body.len().saturating_sub(MAX_PAYLOAD_LEN_IN_BYTE)
        );
    }

    let payload = body[MAX_PAYLOAD_LEN_IN_BYTE..].to_vec();

    Ok(BrokerMessage::Broadcast { payload })
}

fn decode_client_input(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + CLIENT_INPUT_LEN {
        anyhow::bail!("invalid ClientInput length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);

    let mut input = [0_u8; CLIENT_INPUT_LEN];
    input.copy_from_slice(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + CLIENT_INPUT_LEN]);

    Ok(BrokerMessage::ClientInput { client_id, input })
}

fn decode_add_client_to_shard(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() < TOPIC_LEN + CLIENT_ID_LEN + MAX_PAYLOAD_LEN_IN_BYTE {
        anyhow::bail!("AddClientToShard too short: {}", body.len());
    }

    let topic = read_topic(&body[0..TOPIC_LEN]);
    let client_id_start = TOPIC_LEN;
    let client_id_end = client_id_start + CLIENT_ID_LEN;
    let payload_len_start = client_id_end;
    let payload_len_end = payload_len_start + MAX_PAYLOAD_LEN_IN_BYTE;

    let client_id = read_u32_le(&body[client_id_start..client_id_end]);
    let payload_len = read_u16_le(&body[payload_len_start..payload_len_end]) as usize;

    let expected_len = TOPIC_LEN + CLIENT_ID_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload_len;

    if body.len() != expected_len {
        anyhow::bail!(
            "invalid AddClientToShard payload length: declared={}, actual={}",
            payload_len,
            body.len().saturating_sub(TOPIC_LEN + CLIENT_ID_LEN + MAX_PAYLOAD_LEN_IN_BYTE)
        );
    }

    let payload = body[payload_len_end..].to_vec();

    Ok(BrokerMessage::AddClientToShard {
        topic,
        client_id,
        payload,
    })
}

fn decode_set_client_authority(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + TOPIC_LEN {
        anyhow::bail!("invalid SetClientAuthority length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);
    let topic = read_topic(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + TOPIC_LEN]);

    Ok(BrokerMessage::SetClientAuthority { client_id, topic })
}

fn decode_client_hello(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if !body.is_empty() {
        anyhow::bail!("invalid ClientHello length: {}", body.len());
    }
    let username = String::from_utf8(body.to_vec()).map_err(|_| anyhow::anyhow!("invalid username encoding"))?;

    Ok(BrokerMessage::ClientHello{username})
}

fn decode_client_accepted(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN {
        anyhow::bail!("invalid ClientAccepted length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);

    Ok(BrokerMessage::ClientAccepted { client_id })
}


