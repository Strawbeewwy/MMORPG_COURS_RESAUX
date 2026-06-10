
use crate::protocol::{EntityId, EntityType, NetVec2};
use crate::protocol::utils::utils::{
    read_entity_id, read_entity_type, read_net_vec2,
    write_entity_id, write_entity_type, write_net_vec2,
    BinaryDecode, BinaryEncode
};

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySnapshot {
    pub entity_id: EntityId,
    pub entity_type: EntityType,
    pub position: NetVec2,
    pub velocity: NetVec2,
}



impl BinaryEncode for EntitySnapshot {
    fn encode_binary(&self, output: &mut Vec<u8>) -> anyhow::Result<()> {
        write_entity_id(output, self.entity_id);
        write_entity_type(output, self.entity_type);
        write_net_vec2(output, self.position);
        write_net_vec2(output, self.velocity);

        Ok(())
    }
}

impl BinaryDecode for EntitySnapshot {
    fn decode_binary(input: &mut &[u8]) -> anyhow::Result<Self> {
        let entity_id = read_entity_id(input)?;
        let entity_type = read_entity_type(input)?;
        let position = read_net_vec2(input)?;
        let velocity = read_net_vec2(input)?;

        Ok(EntitySnapshot {
            entity_id,
            entity_type,
            position,
            velocity,
        })
    }
}