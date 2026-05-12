/**
login_system contains the systems that are used to draw the
systems UI.
It also polls the systems task to check if the systems process
is complete.
**/


use bevy::prelude::*;
use tokio::sync::oneshot;

use crate::net::gatekeeper::login_to_gatekeeper;
use crate::protocol::LoginResponse;
use crate::resources::network_resources::{
    LoginRequestMessage, LoginStatus, LoginTask, TokioRuntimeResource,
};


/**
This plugin adds the systems as update so that they are run
each frame.
**/
pub struct LoginSystemPlugin;

impl Plugin for LoginSystemPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, poll_login_task)
            .add_systems(Update, login_trigger_system);

    }
}


pub fn login_trigger_system(
    mut messages: MessageReader<LoginRequestMessage>,
    mut login_status: ResMut<LoginStatus>,
    mut login_task: ResMut<LoginTask>,
    tokio_runtime: Res<TokioRuntimeResource>,
) {
    for message in messages.read() {
        start_login(
            &message.username,
            &message.password,
            &mut *login_status,
            &mut *login_task,
            &tokio_runtime,
        );
    }
}

pub fn start_login(
    username: &str,
    password: &str,
    login_status: &mut LoginStatus,
    login_task: &mut LoginTask,
    tokio_runtime: &TokioRuntimeResource,
) {
    if username.trim().is_empty() {
        *login_status = LoginStatus::Error {
            message: "Username is required.".to_string(),
        };
        return;
    }

    if password.is_empty() {
        *login_status = LoginStatus::Error {
            message: "Password is required.".to_string(),
        };
        return;
    }

    let username = username.to_string();
    let password = password.to_string();

    let (sender, receiver) = oneshot::channel();

    tokio_runtime.runtime.spawn(async move {
        let result = login_to_gatekeeper(&username, &password).await;
        let _ = sender.send(result);
    });

    login_task.receiver = Some(receiver);
    *login_status = LoginStatus::LoggingIn;
}

fn poll_login_task(
    mut login_task: ResMut<LoginTask>,
    mut login_status: ResMut<LoginStatus>,
) {
    let Some(receiver) = login_task.receiver.as_mut() else {
        return;
    };

    match receiver.try_recv() {
        Ok(result) => {
            login_task.receiver = None;

            match result {
                Ok(response) => match response {
                    LoginResponse::Success { session_token, game_server_address } => {
                        *login_status = LoginStatus::Success {
                            session_token,
                            game_server_address,
                        };
                    }
                    LoginResponse::Failed { reason } => {
                        *login_status = LoginStatus::Failed { reason };
                    }
                    LoginResponse::ServerFull { queue_position } => {
                        let reason = format!("Server is full. Please try again later. Queue position: {}", queue_position);
                        *login_status = LoginStatus::Failed { reason };
                    }
                },
                Err(error) => {
                    *login_status = LoginStatus::Error {
                        message: error.to_string(),
                    };
                }
            }
        }
        Err(oneshot::error::TryRecvError::Empty) => {}
        Err(oneshot::error::TryRecvError::Closed) => {
            login_task.receiver = None;
            *login_status = LoginStatus::Error {
                message: "Login task was cancelled.".to_string(),
            };
        }
    }
}