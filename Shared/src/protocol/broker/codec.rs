pub const TAG_SUBSCRIBE: u8 = 0x01;
pub const TAG_UNSUBSCRIBE: u8 = 0x02;
pub const TAG_PUBLISH: u8 = 0x03;
pub const TAG_BROADCAST: u8 = 0x04;
pub const TAG_CLIENT_INPUT: u8 = 0x05;
pub const TAG_REGISTER_CLIENT: u8 = 0x06;
pub const TAG_REGISTER_SHARD: u8 = 0x07;
pub const TAG_REGISTER_SPATIAL_SERVICE: u8 = 0x08;
pub const TAG_ADD_CLIENT_TO_SHARD: u8 = 0x09;
pub const TAG_SET_CLIENT_AUTHORITY: u8 = 0x0A;
pub const TAG_CLIENT_HELLO: u8 = 0x0B;
pub const TAG_CLIENT_ACCEPTED: u8 = 0x0C;

pub const TAG_LEN: usize = size_of::<u8>();
pub const MAX_PAYLOAD_LEN_IN_BYTE: usize = size_of::<u16>();
pub const TOPIC_LEN: usize = 32;
pub const CLIENT_INPUT_LEN: usize = 16;

pub type ClientId = u32;
pub const CLIENT_ID_LEN: usize = size_of::<ClientId>();
pub type Topic = [u8; TOPIC_LEN];

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
    ClientHello,
    ClientAccepted {
        client_id: ClientId,
    },
}

pub fn encode_register_client(client_id: ClientId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    packet.push(TAG_REGISTER_CLIENT);
    packet.extend_from_slice(&client_id.to_le_bytes());

    packet
}

pub fn encode_register_shard(topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN);

    packet.push(TAG_REGISTER_SHARD);
    packet.extend_from_slice(&topic);

    packet
}

pub fn encode_register_spatial_service() -> Vec<u8> {
    vec![TAG_REGISTER_SPATIAL_SERVICE]
}

pub fn encode_subscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    packet.push(TAG_SUBSCRIBE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}


pub fn encode_unsubscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    packet.push(TAG_UNSUBSCRIBE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}

pub fn encode_publish(topic: Topic, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload.len());

    packet.push(TAG_PUBLISH);
    packet.extend_from_slice(&topic);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

pub fn encode_broadcast(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload.len());

    packet.push(TAG_BROADCAST);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

pub fn encode_client_input(
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + CLIENT_INPUT_LEN);

    packet.push(TAG_CLIENT_INPUT);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&input);

    packet
}

pub fn encode_add_client_to_shard(
    topic: Topic,
    client_id: ClientId,
    payload: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN + CLIENT_ID_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload.len());

    packet.push(TAG_ADD_CLIENT_TO_SHARD);
    packet.extend_from_slice(&topic);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

pub fn encode_set_client_authority(
    client_id: ClientId,
    topic: Topic,
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    packet.push(TAG_SET_CLIENT_AUTHORITY);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}

pub fn encode_client_hello() -> Vec<u8> {
    vec![TAG_CLIENT_HELLO]
}

pub fn encode_client_accepted(client_id: ClientId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    packet.push(TAG_CLIENT_ACCEPTED);
    packet.extend_from_slice(&client_id.to_le_bytes());

    packet
}

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
        unknown => anyhow::bail!("unknown broker message tag: 0x{unknown:02x}"),
    }
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

    Ok(BrokerMessage::ClientHello)
}

fn decode_client_accepted(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != CLIENT_ID_LEN {
        anyhow::bail!("invalid ClientAccepted length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..CLIENT_ID_LEN]);

    Ok(BrokerMessage::ClientAccepted { client_id })
}

fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn read_topic(bytes: &[u8]) -> Topic {
    let mut topic = [0_u8; TOPIC_LEN];
    topic.copy_from_slice(bytes);
    topic
}

/// Build a Topic from a shard id (e.g. 0 → "shard:0").
/// Uses a small inline buffer to avoid heap allocation on every call.
pub fn topic_for_shard(shard_id: u32) -> Topic {
    let mut topic = [0u8; TOPIC_LEN];
    // Write "shard:" prefix then the decimal digits directly into the buffer.
    let prefix = b"shard:";
    topic[..prefix.len()].copy_from_slice(prefix);
    let mut n = shard_id;
    let mut digits = [0u8; 10]; // u32::MAX is 10 digits
    let mut len = 0usize;
    if n == 0 {
        digits[0] = b'0';
        len = 1;
    } else {
        while n > 0 {
            digits[len] = b'0' + (n % 10) as u8;
            n /= 10;
            len += 1;
        }
        digits[..len].reverse();
    }
    topic[prefix.len()..prefix.len() + len].copy_from_slice(&digits[..len]);
    topic
}

pub fn topic_from_str(value: &str) -> Topic {
    let mut topic = [0_u8; TOPIC_LEN];
    let bytes = value.as_bytes();
    let len = bytes.len().min(TOPIC_LEN);

    topic[..len].copy_from_slice(&bytes[..len]);

    topic
}

pub fn topic_to_string(topic: &Topic) -> String {
    let len = topic
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(TOPIC_LEN);

    String::from_utf8_lossy(&topic[..len]).to_string()
}