/*
login_system contains the systems that are used
It also polls the systems task to check if the systems process
is complete.
*/
use bevy::prelude::*;
use tokio::sync::oneshot;

use crate::net::gatekeeper::login_to_gatekeeper;
use crate::resources::network_resources::{
    LoginRequestMessage, LoginStatus, LoginTask, TokioRuntimeResource,
};
use crate::systems::launch_game_system::LaunchGameClientMessage;

/*
This plugin adds the systems as update so that they are run
each frame.
*/
pub struct LoginSystemPlugin;

impl Plugin for LoginSystemPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, poll_login_task)
            .add_systems(Update, login_trigger_system);
    }
}

/*
go through all the messages and start the login process
if the message is a login request message.
*/
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

/*
start the login process by sending a request to the gatekeeper
server.
*/
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

    //create a oneshot channel to communicate with the systems task
    let (sender, receiver) = oneshot::channel();

    //spawn the login task
    tokio_runtime.runtime.spawn({
        let username = username.clone();

        async move {
            let result = login_to_gatekeeper(&username, &password).await;
            let _ = sender.send(result);
        }
    });
    //store the oneshot channel in the login task so polling can acess it everyframe
    login_task.receiver = Some(receiver);
    login_task.username = Some(username);
    *login_status = LoginStatus::LoggingIn;
}
fn poll_login_task(
    mut login_task: ResMut<LoginTask>,
    mut login_status: ResMut<LoginStatus>,
    mut launch_messages: MessageWriter<LaunchGameClientMessage>,
) {
    //check if we have a task to poll
    let Some(receiver) = login_task.receiver.as_mut() else {
        return;
    };

    match receiver.try_recv() {
        Ok(result) => {
            login_task.receiver = None;

            match result {
                Ok(response) => {
                    let username = login_task.username.clone().unwrap_or_default();

                    launch_messages.write(LaunchGameClientMessage {
                        player_id: response.player_id.clone(),
                        username: username.clone(),
                        server_ip: response.server.ip.clone(),
                        server_port: response.server.port,
                        zone: response.server.zone.clone(),
                    });

                    *login_status = LoginStatus::Success {
                        player_id: response.player_id,
                        server_ip: response.server.ip,
                        server_port: response.server.port,
                        zone: response.server.zone,
                        username,
                    };
                }
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