pub const TAG_SUBSCRIBE: u8 = 0x01;
pub const TAG_UNSUBSCRIBE: u8 = 0x02;
pub const TAG_PUBLISH: u8 = 0x03;
pub const TAG_BROADCAST: u8 = 0x04;
pub const TAG_CLIENT_INPUT: u8 = 0x05;

pub const TOPIC_LEN: usize = 32;
pub const CLIENT_INPUT_LEN: usize = 16;

pub type ClientId = u32;
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
}

pub fn encode_subscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(1 + 4 + TOPIC_LEN);

    packet.push(TAG_SUBSCRIBE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}

pub fn encode_unsubscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(1 + 4 + TOPIC_LEN);

    packet.push(TAG_UNSUBSCRIBE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}

pub fn encode_publish(topic: Topic, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(1 + TOPIC_LEN + 2 + payload.len());

    packet.push(TAG_PUBLISH);
    packet.extend_from_slice(&topic);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

pub fn encode_broadcast(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(1 + 2 + payload.len());

    packet.push(TAG_BROADCAST);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

pub fn encode_client_input(
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(1 + 4 + CLIENT_INPUT_LEN);

    packet.push(TAG_CLIENT_INPUT);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&input);

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
        unknown => anyhow::bail!("unknown broker message tag: 0x{unknown:02x}"),
    }
}

fn decode_subscribe(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != 4 + TOPIC_LEN {
        anyhow::bail!("invalid Subscribe length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..4]);
    let topic = read_topic(&body[4..4 + TOPIC_LEN]);

    Ok(BrokerMessage::Subscribe { client_id, topic })
}

fn decode_unsubscribe(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != 4 + TOPIC_LEN {
        anyhow::bail!("invalid Unsubscribe length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..4]);
    let topic = read_topic(&body[4..4 + TOPIC_LEN]);

    Ok(BrokerMessage::Unsubscribe { client_id, topic })
}

fn decode_publish(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() < TOPIC_LEN + 2 {
        anyhow::bail!("Publish too short: {}", body.len());
    }

    let topic = read_topic(&body[0..TOPIC_LEN]);
    let payload_len_start = TOPIC_LEN;
    let payload_len_end = TOPIC_LEN + 2;
    let payload_len = read_u16_le(&body[payload_len_start..payload_len_end]) as usize;

    let expected_len = TOPIC_LEN + 2 + payload_len;

    if body.len() != expected_len {
        anyhow::bail!(
            "invalid Publish payload length: declared={}, actual={}",
            payload_len,
            body.len().saturating_sub(TOPIC_LEN + 2)
        );
    }

    let payload = body[payload_len_end..].to_vec();

    Ok(BrokerMessage::Publish { topic, payload })
}

fn decode_broadcast(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() < 2 {
        anyhow::bail!("Broadcast too short: {}", body.len());
    }

    let payload_len = read_u16_le(&body[0..2]) as usize;
    let expected_len = 2 + payload_len;

    if body.len() != expected_len {
        anyhow::bail!(
            "invalid Broadcast payload length: declared={}, actual={}",
            payload_len,
            body.len().saturating_sub(2)
        );
    }

    let payload = body[2..].to_vec();

    Ok(BrokerMessage::Broadcast { payload })
}

fn decode_client_input(body: &[u8]) -> anyhow::Result<BrokerMessage> {
    if body.len() != 4 + CLIENT_INPUT_LEN {
        anyhow::bail!("invalid ClientInput length: {}", body.len());
    }

    let client_id = read_u32_le(&body[0..4]);

    let mut input = [0_u8; CLIENT_INPUT_LEN];
    input.copy_from_slice(&body[4..4 + CLIENT_INPUT_LEN]);

    Ok(BrokerMessage::ClientInput { client_id, input })
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