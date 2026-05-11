use bevy::prelude::*;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::protocol::LoginResponse;

#[derive(Resource)]
pub struct TokioRuntimeResource {
    pub runtime: Runtime,
}

impl TokioRuntimeResource {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("failed to create Tokio runtime"),
        }
    }
}

#[derive(Resource, Default)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Resource, Debug, Clone)]
pub enum LoginStatus {
    Idle,
    LoggingIn,
    Success {
        session_token: String,
        game_server_address: String,
    },
    Failed {
        reason: String,
    },
    Error {
        message: String,
    },
}

impl Default for LoginStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Resource, Default)]
pub struct LoginTask {
    pub receiver: Option<oneshot::Receiver<anyhow::Result<LoginResponse>>>,
}

pub struct LoginResourcesPlugin;

impl Plugin for LoginResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TokioRuntimeResource::new())
            .init_resource::<LoginForm>()
            .init_resource::<LoginStatus>()
            .init_resource::<LoginTask>();
    }
}