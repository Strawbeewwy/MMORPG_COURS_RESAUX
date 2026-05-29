
use crate::protocol::broker::config::*;

pub use crate::protocol::broker::broker_message::{
    CLIENT_ID_LEN, ClientId,BrokerMessage,
};
use crate::protocol::broker::broker_message::read_client_id;
pub use crate::protocol::broker::topic::{
    ShardId,Topic,TOPIC_LEN, read_topic,
};

pub use crate::protocol::broker::utils::{
    read_u32_le, read_u16_le,
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
        TAG_REGISTER_SHARD => decode_register_shard(body),
        TAG_REGISTER_SPATIAL_SERVICE => decode_register_spatial_service(body),
        TAG_CLIENT_HELLO => decode_client_hello(body),
        TAG_CLIENT_ACCEPTED => decode_client_accepted(body),
        TAG_POSITION_UPDATE => decode_position_update(body),
        unknown => anyhow::bail!("unknown broker message tag: 0x{unknown:02x}"),
    }
}

fn decode_position_update(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + 2 * size_of::<f32>() {
        anyhow::bail!("invalid PositionUpdate length: {}", body.len());
    }

    let client_id = read_client_id(&body[0..CLIENT_ID_LEN]);

    let x_start = CLIENT_ID_LEN;
    let x_end = x_start + size_of::<f32>();
    let y_start = x_end;
    let y_end = y_start + size_of::<f32>();

    let position = [
        f32::from_le_bytes(body[x_start..x_end].try_into()?),
        f32::from_le_bytes(body[y_start..y_end].try_into()?),
    ];
    Ok(BrokerMessage::PositionUpdate { client_id, position })
}

fn decode_register_shard(body: &[u8]) -> anyhow::Result<BrokerMessage> {
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

    let client_id = read_client_id(&body[0..CLIENT_ID_LEN]);
    let topic = read_topic(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + TOPIC_LEN]);

    if let Topic::ShardInstance(shard_id) = topic {
        Ok(BrokerMessage::Subscribe {client_id, shard_id })
    } else {
        anyhow::bail!("Topic received is not a valid Shard instance")
    }
}

fn decode_unsubscribe(body: &[u8]) -> anyhow::Result<BrokerMessage> {
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

fn decode_publish(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() > MAX_PAYLOAD_LEN {
        anyhow::bail!("Publish too short: {}", body.len());
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

    let payload = body[TOPIC_LEN + size_of::<u16>()..].to_vec();

    if let Topic::ShardInstance(shard_id) = topic {
        Ok(BrokerMessage::Publish { shard_id, payload })
    } else {
        anyhow::bail!("Topic received is not a valid Shard instance")
    }
}

fn decode_broadcast(body: &[u8]) -> anyhow::Result<BrokerMessage> {
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

    let payload = body[size_of::<u16>()..].to_vec();

    Ok(BrokerMessage::Broadcast { payload })
}

fn decode_client_input(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN + CLIENT_INPUT_LEN {
        anyhow::bail!("invalid ClientInput length: {}", body.len());
    }

    let client_id = ClientId(read_u32_le(&body[0..CLIENT_ID_LEN]));

    let mut input = [0_u8; CLIENT_INPUT_LEN];
    input.copy_from_slice(&body[CLIENT_ID_LEN..CLIENT_ID_LEN + CLIENT_INPUT_LEN]);

    Ok(BrokerMessage::ClientInput { client_id, input })
}

fn decode_client_hello(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    // body carries the UTF-8 encoded username; empty username is invalid.
    if body.is_empty() {
        anyhow::bail!("invalid ClientHello: empty username");
    }
    let username = String::from_utf8(body.to_vec())
        .map_err(|_| anyhow::anyhow!("invalid ClientHello: non-UTF8 username"))?;
    Ok(BrokerMessage::ClientHello { username })
}

fn decode_client_accepted(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN {
        anyhow::bail!("invalid ClientAccepted length: {}", body.len());
    }

    let client_id = ClientId(read_u32_le(&body[0..CLIENT_ID_LEN]));

    Ok(BrokerMessage::ClientAccepted { client_id })
}


