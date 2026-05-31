use bevy::prelude::*;
use game_sockets::protocols::QuicBackend;
use game_sockets::GamePeer;
use std::collections::HashMap;
use crate::resources::config::SpatialConfig;
use crate::resources::net_handles::ShardListener;

/// Startup system — bind the QUIC listener that shards will connect to.
pub fn bind_shard_listener(mut commands: Commands, config: Res<SpatialConfig>) {
    let peer = GamePeer::new(QuicBackend::new());

    peer.listen(&config.listen_host, config.listen_port)
        .expect("failed to bind shard listener");

    tracing::info!(
        "spatial: shard listener bound on {}:{}",
        config.listen_host,
        config.listen_port
    );

    commands.insert_resource(ShardListener {
        peer,
        streams: HashMap::new(),
        connection_by_shard_id: HashMap::new(),
        shard_id_by_connection: HashMap::new(),
    });
}

