use crate::protocol::broker::config::*;

pub use crate::protocol::broker::broker_message::{
    CLIENT_ID_LEN, TOPIC_LEN, ClientId, Topic, BrokerMessage,
};


pub fn encode_message(message: &BrokerMessage) -> anyhow::Result<Vec<u8>> {
    match message {
        BrokerMessage::Subscribe { client_id, topic } => {
            Ok(encode_subscribe(*client_id, *topic))
        }

        BrokerMessage::Unsubscribe { client_id, topic } => {
            Ok(encode_unsubscribe(*client_id, *topic))
        }

        BrokerMessage::Publish { topic, payload } => {
            encode_publish(*topic, payload)
        }

        BrokerMessage::Broadcast { payload } => {
            encode_broadcast(payload)
        }

        BrokerMessage::ClientInput { client_id, input } => {
            Ok(encode_client_input(*client_id, *input))
        }

        BrokerMessage::RegisterClient { client_id } => {
            Ok(encode_register_client(*client_id))
        }

        BrokerMessage::RegisterShard { topic } => {
            Ok(encode_register_shard(*topic))
        }

        BrokerMessage::RegisterSpatialService => {
            Ok(encode_register_spatial_service())
        }

        BrokerMessage::AddClientToShard {
            topic,
            client_id,
            payload,
        } => {
            encode_add_client_to_shard(*topic, *client_id, payload)
        }

        BrokerMessage::SetClientAuthority { client_id, topic } => {
            Ok(encode_set_client_authority(*client_id, *topic))
        }

        BrokerMessage::ClientHello{username} => {
            Ok(encode_client_hello(username.clone()))
        }

        BrokerMessage::ClientAccepted { client_id } => {
            Ok(encode_client_accepted(*client_id))
        },
        BrokerMessage::PositionUpdate {
            client_id,
            position,
        } => {
            Ok(encode_position_update(*client_id, position))
        },
    }
}

fn encode_position_update(client_id: ClientId, positions: &[f32; 2]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + 2 * size_of::<f32>());

    packet.push(TAG_POSITION_UPDATE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&positions[0].to_le_bytes());
    packet.extend_from_slice(&positions[1].to_le_bytes());

    packet
}

fn encode_register_client(client_id: ClientId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    packet.push(TAG_REGISTER_CLIENT);
    packet.extend_from_slice(&client_id.to_le_bytes());

    packet
}

fn encode_register_shard(topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN);

    packet.push(TAG_REGISTER_SHARD);
    packet.extend_from_slice(&topic);

    packet
}

fn encode_register_spatial_service() -> Vec<u8> {
    vec![TAG_REGISTER_SPATIAL_SERVICE]
}

fn encode_subscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    packet.push(TAG_SUBSCRIBE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}


fn encode_unsubscribe(client_id: ClientId, topic: Topic) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    packet.push(TAG_UNSUBSCRIBE);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}

fn encode_publish(topic: Topic, payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + TOPIC_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload.len());

    packet.push(TAG_PUBLISH);
    packet.extend_from_slice(&topic);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_broadcast(payload: &[u8]) -> anyhow::Result<Vec<u8>> {
    let payload_len = u16::try_from(payload.len())?;

    let mut packet = Vec::with_capacity(TAG_LEN + MAX_PAYLOAD_LEN_IN_BYTE + payload.len());

    packet.push(TAG_BROADCAST);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(payload);

    Ok(packet)
}

fn encode_client_input(
    client_id: ClientId,
    input: [u8; CLIENT_INPUT_LEN],
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + CLIENT_INPUT_LEN);

    packet.push(TAG_CLIENT_INPUT);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&input);

    packet
}

fn encode_add_client_to_shard(
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

fn encode_set_client_authority(
    client_id: ClientId,
    topic: Topic,
) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN + TOPIC_LEN);

    packet.push(TAG_SET_CLIENT_AUTHORITY);
    packet.extend_from_slice(&client_id.to_le_bytes());
    packet.extend_from_slice(&topic);

    packet
}

fn encode_client_hello(
    username: String,
) -> Vec<u8> {
    let username_bytes = username.as_bytes();

    let mut packet = Vec::with_capacity(
        TAG_LEN + size_of::<u16>() + username_bytes.len()
    );

    packet.push(TAG_CLIENT_HELLO);
    packet.extend_from_slice(&(username_bytes.len() as u16).to_be_bytes());
    packet.extend_from_slice(username_bytes);

    packet
}

fn encode_client_accepted(client_id: ClientId) -> Vec<u8> {
    let mut packet = Vec::with_capacity(TAG_LEN + CLIENT_ID_LEN);

    packet.push(TAG_CLIENT_ACCEPTED);
    packet.extend_from_slice(&client_id.to_le_bytes());

    packet
}