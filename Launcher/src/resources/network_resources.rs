use bevy::prelude::*;
use shared::protocol::LoginHttpResponse;
use tokio::runtime::Runtime;
use tokio::sync::oneshot;

/**
This declares a Tokio runtime as a resource.
**/
#[derive(Resource)]
pub struct TokioRuntimeResource {
    pub runtime: Runtime,
}

/**
This implements a Tokio runtime to run our async code.
**/
impl TokioRuntimeResource {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new().expect("failed to create Tokio runtime"),
        }
    }
}

/**
This declares a login status resource so that the login
status information is persistent across frames.
**/
#[derive(Resource, Debug, Clone)]
pub enum LoginStatus {
    Idle,
    LoggingIn,
    Success {
        client_id: String,
        username: String,
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

/*
This resource is used to store the oneshot channel that awaits
the result of the systems process. The bevy app is the receiver,
while the sender is the gatekeeper.
*/
#[derive(Resource, Default)]
pub struct LoginTask {
    pub receiver: Option<oneshot::Receiver<anyhow::Result<LoginHttpResponse>>>,
    pub username: Option<String>,
}

/*
This declares a message sent to the login system, so it
what the username and password to log in with. Since the login form
is a ressouce and can be modified whenever, we make sure to send
this message so that the login system uses the latest information
and not a resource that might be outdated.
*/
#[derive(Message, Debug, Clone)]
pub struct LoginRequestMessage {
    pub username: String,
    pub password: String,
}

pub struct NetworkingResourcesPlugin;
impl Plugin for NetworkingResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TokioRuntimeResource::new())
            .init_resource::<LoginStatus>()
            .init_resource::<LoginTask>()
            .add_message::<LoginRequestMessage>();
    }
}
