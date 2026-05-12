
use bevy::prelude::*;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

use crate::protocol::LoginResponse;

/**
This declares a Tokio runtime as a resource.
**/
#[derive(Resource)]
pub struct TokioRuntimeResource {
    pub runtime: Runtime,
}

/**
This implements a multithreaded Tokio runtime to run our
async code.
**/
impl TokioRuntimeResource {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("failed to create Tokio runtime"),
        }
    }
}

/**
This declares a systems form resource so that the systems
information is persistent across frames.
**/
#[derive(Resource, Default)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/**
This declares a systems status resource so that the systems
status information is persistent across frames.
**/
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

/**
the default value for LoginStatus is Idle since no username
and password have been entered yet.
**/
impl Default for LoginStatus {
    fn default() -> Self {
        Self::Idle
    }
}


/**
This resource is used to store the oneshot channel that awaits
the result of the systems process. The bevy app is the receiver,
while the sender is the gatekeeper.
**/
#[derive(Resource, Default)]
pub struct LoginTask {
    pub receiver: Option<oneshot::Receiver<anyhow::Result<LoginResponse>>>,
}

pub struct NetworkingResources;
impl Plugin for NetworkingResources {
    fn build(&self, app: &mut App) {
        app.insert_resource(TokioRuntimeResource::new())
            .init_resource::<LoginForm>()
            .init_resource::<LoginStatus>()
            .init_resource::<LoginTask>();
    }
}