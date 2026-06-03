use crate::config::ServerConfig;
use crate::net::network_event::SharedPlayerRegistry;
use bevy::prelude::*;
use shared::protocol::http::codec;
use shared::protocol::Heartbeat;
use std::net::UdpSocket;
use std::time::Duration;

#[derive(Resource)]
pub struct HeartbeatSocket {
    pub socket: UdpSocket,
}

#[derive(Resource)]
pub struct HeartbeatTimer {
    pub timer: Timer,
}

pub fn bind_heartbeat_socket(mut commands: Commands) {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .expect("failed to bind UDP heartbeat socket");

    socket
        .set_nonblocking(true)
        .expect("failed to set heartbeat socket as non-blocking");

    commands.insert_resource(HeartbeatSocket { socket });

    commands.insert_resource(HeartbeatTimer {
        timer: Timer::new(Duration::from_secs(5), TimerMode::Repeating),
    });
}

pub fn send_heartbeat(
    time: Res<Time>,
    socket: Res<HeartbeatSocket>,
    config: Res<ServerConfig>,
    registry: Res<SharedPlayerRegistry>,
    mut heartbeat_timer: ResMut<HeartbeatTimer>,
) {
    heartbeat_timer.timer.tick(time.delta());

    if !heartbeat_timer.timer.just_finished() {
        return;
    }

    let player_count = match registry.inner.try_lock() {
        Ok(registry) => registry.player_count(),
        Err(_) => {
            tracing::warn!("could not lock player registry for heartbeat");
            return;
        }
    };

    let heartbeat = Heartbeat {
        id: config.shard_topic.to_string(),
        ip: config.ip.clone(),
        port: config.port,
        zone: config.zone.clone(),
        player_count,
        max_players: config.max_players,
    };

    let payload = match codec::encode(&heartbeat) {
        Ok(payload) => payload,
        Err(error) => {
            tracing::error!("failed to encode heartbeat: {error:#}");
            return;
        }
    };

    match socket
        .socket
        .send_to(&payload, config.orchestrator_addr)
    {
        Ok(bytes) => {
            tracing::info!(
                "sent heartbeat to {} bytes={} players={}/{} status={}",
                config.orchestrator_addr,
                bytes,
                heartbeat.player_count,
                heartbeat.max_players,
                heartbeat.status()
            );
        }
        Err(error) => {
            tracing::error!(
                "failed to send heartbeat to {}: {error}",
                config.orchestrator_addr
            );
        }
    }
}