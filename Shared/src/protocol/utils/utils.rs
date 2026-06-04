use std::sync::Arc;
use anyhow::Context;
use crate::protocol::{ClientId, NetVec2, Username};
use crate::protocol::game::PlayerId;
pub use crate::protocol::message::network_message::{
    NetworkMessage,
};

const U16_LEN: usize = size_of::<u16>();
const U32_LEN: usize = size_of::<u32>();
const U64_LEN: usize = size_of::<u64>();
const U128_LEN: usize = size_of::<u128>();
const NET_VEC2_LEN: usize = 10;



pub trait BinaryEncode {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()>;
}

pub trait BinaryDecode: Sized {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self>;
}

pub fn encode<T: BinaryEncode>(message: &T) -> anyhow::Result<Vec<u8>> {
    let mut output = Vec::new();
    message
        .encode_binary(&mut output)
        .context("failed to encode protocol message")?;
    Ok(output)
}

pub fn decode<T: BinaryDecode>(bytes: &[u8]) -> anyhow::Result<T> {
    let mut input = bytes;
    let decoded = T::decode_binary(&mut input).context("failed to decode protocol message")?;

    if !input.is_empty() {
        anyhow::bail!("trailing bytes after protocol message: {}", input.len());
    }

    Ok(decoded)
}

pub fn write_u8(output: &mut Vec<u8>, value: u8) {
    output.push(value);
}

pub fn write_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub fn write_u128(output: &mut Vec<u8>, value: u128) {
    output.extend_from_slice(&value.to_le_bytes());
}

pub fn write_client_id(output: &mut Vec<u8>, client_id: ClientId) {
    write_u32(output, client_id.0);
}

pub fn write_player_id(output: &mut Vec<u8>, player_id: PlayerId) {
    write_u128(output, player_id);
}

pub fn write_net_vec2(output: &mut Vec<u8>, value: NetVec2) {
    output.extend_from_slice(&value.to_bytes());
}

pub fn write_username(output: &mut Vec<u8>, username: &Username) -> anyhow::Result<()> {
    write_arc_str(output, username)
}

pub fn write_arc_str(output: &mut Vec<u8>, value: &Arc<str>) -> anyhow::Result<()> {
    let bytes = value.as_bytes();
    write_len_u16(output, bytes.len(), "string length")?;
    output.extend_from_slice(bytes);

    Ok(())
}

pub fn write_len_u16(output: &mut Vec<u8>, len: usize, name: &str) -> anyhow::Result<()> {
    let len = u16::try_from(len)
        .with_context(|| format!("{name} does not fit in u16: {len}"))?;
    write_u16(output, len);

    Ok(())
}

pub fn read_u8(input: &mut &[u8]) -> anyhow::Result<u8> {
    let bytes = take(input, 1)?;
    Ok(bytes[0])
}

pub fn read_u16(input: &mut &[u8]) -> anyhow::Result<u16> {
    let bytes = take(input, U16_LEN)?;
    Ok(u16::from_le_bytes(bytes.try_into()?))
}

pub fn read_u32(input: &mut &[u8]) -> anyhow::Result<u32> {
    let bytes = take(input, U32_LEN)?;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}

pub fn read_u64(input: &mut &[u8]) -> anyhow::Result<u64> {
    let bytes = take(input, U64_LEN)?;
    Ok(u64::from_le_bytes(bytes.try_into()?))
}

pub fn read_u128(input: &mut &[u8]) -> anyhow::Result<u128> {
    let bytes = take(input, U128_LEN)?;
    Ok(u128::from_le_bytes(bytes.try_into()?))
}

pub fn read_client_id(input: &mut &[u8]) -> anyhow::Result<ClientId> {
    Ok(ClientId(read_u32(input)?))
}

pub fn read_player_id(input: &mut &[u8]) -> anyhow::Result<PlayerId> {
    read_u128(input)
}

pub fn read_net_vec2(input: &mut &[u8]) -> anyhow::Result<NetVec2> {
    let bytes = take(input, NET_VEC2_LEN)?;
    let mut net_vec2_bytes = [0_u8; NET_VEC2_LEN];
    net_vec2_bytes.copy_from_slice(bytes);

    NetVec2::try_from(net_vec2_bytes)
        .map_err(|error| anyhow::anyhow!("invalid NetVec2: {error}"))
}

pub fn read_username(input: &mut &[u8]) -> anyhow::Result<Username> {
    read_arc_str(input)
}

pub fn read_arc_str(input: &mut &[u8]) -> anyhow::Result<Arc<str>> {
    let len = read_u16(input)? as usize;
    let bytes = take(input, len)?;

    let value = std::str::from_utf8(bytes)
        .context("invalid UTF-8 string in protocol message")?;

    Ok(Arc::from(value))
}

fn take<'a>(input: &mut &'a [u8], len: usize) -> anyhow::Result<&'a [u8]> {
    if input.len() < len {
        anyhow::bail!(
            "not enough bytes: need {}, remaining {}",
            len,
            input.len()
        );
    }

    let (head, tail) = input.split_at(len);
    *input = tail;

    Ok(head)
}

pub fn read_exact<'a>(input: &mut &'a [u8], len: usize) -> anyhow::Result<&'a [u8]> {
    if input.len() < len {
        anyhow::bail!(
            "not enough bytes: need {}, remaining {}",
            len,
            input.len()
        );
    }

    let (head, tail) = input.split_at(len);
    *input = tail;

    Ok(head)
}