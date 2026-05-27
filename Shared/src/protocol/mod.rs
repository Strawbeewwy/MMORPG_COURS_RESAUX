pub mod broker;
pub mod discovery;
pub mod game;
pub mod http;
pub mod spatial;
pub mod transport;

pub use discovery::{Heartbeat, ServerInfo};
pub use game::{
    ClientId, EntityId, PlayerId, PlayerPublicInfo, PlayerSnapshot, PlayerSpawnInfo,
    Username, NetVec2, WorldSnapshot, WorldUpdate, ZoneId,
};
pub use http::{ErrorResponse, HealthResponse, LoginHttpRequest, LoginHttpResponse};
pub use broker::{
    BrokerMessage, Topic, CLIENT_INPUT_LEN, TOPIC_LEN,
    decode_message, encode_add_client_to_shard, encode_broadcast, encode_client_accepted,
    encode_client_hello, encode_client_input, encode_publish, encode_register_client,
    encode_register_shard, encode_register_spatial_service, encode_set_client_authority,
    encode_subscribe, encode_unsubscribe, topic_for_shard, topic_from_str, topic_to_string,
};

