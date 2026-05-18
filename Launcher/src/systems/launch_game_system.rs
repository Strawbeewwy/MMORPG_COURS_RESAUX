use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WinitWindows;
use std::process::{Child, Command, Stdio};

#[derive(Message, Debug, Clone)]
pub struct LaunchGameClientMessage {
    pub player_id: String,
    pub username: String,
    pub server_ip: String,
    pub server_port: u16,
    pub zone: String,
}

#[derive(Resource, Default)]
pub struct GameLaunchState {
    pub launched: bool,
    pub child: Option<Child>,
}

pub struct LaunchGameSystemPlugin;

impl Plugin for LaunchGameSystemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLaunchState>()
            .add_message::<LaunchGameClientMessage>()
            .add_systems(
                Update,
                (
                    launch_game_client_on_message,
                    restore_launcher_when_game_client_exits,
                ),
            );
    }
}

fn launch_game_client_on_message(
    mut messages: MessageReader<LaunchGameClientMessage>,
    mut launch_state: ResMut<GameLaunchState>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    if launch_state.launched {
        return;
    }

    for message in messages.read() {
        let Some(child) = launch_game_client(message) else {
            break;
        };

        launch_state.launched = true;
        launch_state.child = Some(child);

        minimize_launcher_window(primary_window_query, winit_windows);

        break;
    }
}

fn launch_game_client(message: &LaunchGameClientMessage) -> Option<Child> {
    tracing::info!(
        "launching GameClient for username={} player_id={} server={}:{} zone={}",
        message.username,
        message.player_id,
        message.server_ip,
        message.server_port,
        message.zone
    );

    let spawn_result = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("game_client")
        .env("PLAYER_ID", &message.player_id)
        .env("USERNAME", &message.username)
        .env("GAME_SERVER_IP", &message.server_ip)
        .env("GAME_SERVER_PORT", message.server_port.to_string())
        .env("GAME_SERVER_ZONE", &message.zone)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    match spawn_result {
        Ok(child) => {
            tracing::info!("GameClient launched");
            Some(child)
        }
        Err(error) => {
            tracing::error!("failed to launch GameClient: {}", error);
            None
        }
    }
}

fn restore_launcher_when_game_client_exits(
    mut launch_state: ResMut<GameLaunchState>,
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let Some(child) = launch_state.child.as_mut() else {
        return;
    };

    match child.try_wait() {
        Ok(Some(status)) => {
            tracing::info!("GameClient exited with status {}", status);

            launch_state.child = None;
            launch_state.launched = false;

            restore_launcher_window(primary_window_query, winit_windows);
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!("failed to check GameClient process status: {}", error);

            launch_state.child = None;
            launch_state.launched = false;

            restore_launcher_window(primary_window_query, winit_windows);
        }
    }
}

fn minimize_launcher_window(
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let Ok(window_entity) = primary_window_query.single() else {
        tracing::warn!("could not find primary launcher window to minimize");
        return;
    };

    let Some(window) = winit_windows.get_window(window_entity) else {
        tracing::warn!("could not access native launcher window to minimize");
        return;
    };

    window.set_minimized(true);

    tracing::info!("launcher window minimized");
}

fn restore_launcher_window(
    primary_window_query: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let Ok(window_entity) = primary_window_query.single() else {
        tracing::warn!("could not find primary launcher window to restore");
        return;
    };

    let Some(window) = winit_windows.get_window(window_entity) else {
        tracing::warn!("could not access native launcher window to restore");
        return;
    };

    window.set_minimized(false);
    window.set_visible(true);
    window.focus_window();

    tracing::info!("launcher window restored");
}