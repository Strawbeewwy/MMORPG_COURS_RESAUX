use crate::protocol::{ClientId, CLIENT_ID_LEN};
pub use crate::protocol::message::network_message::{
    NetworkMessage,
};


pub fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

pub fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub fn read_client_id(bytes: &[u8]) -> ClientId {

    let mut client_id_bytes = [0_u8; CLIENT_ID_LEN];

    client_id_bytes[..bytes.len()].copy_from_slice(bytes);

    ClientId(read_u32_le(&client_id_bytes))
}
