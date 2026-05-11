/**
systems.rs contains the systems that are used to draw the
login UI.
It also polls the login task to check if the login process
is complete.
**/



use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use tokio::sync::oneshot;

use crate::login::network::login_to_gatekeeper;
use crate::login::resources::{
    LoginForm, LoginStatus, LoginTask, TokioRuntimeResource,
};
use crate::protocol::LoginResponse;


/**
This plugin adds the systems as update so that they are run
each frame.
**/
pub struct LoginSystemsPlugin;

impl Plugin for LoginSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_login_ui, poll_login_task));
    }
}

fn draw_login_ui(
    mut contexts: EguiContexts,
    mut login_form: ResMut<LoginForm>,
    mut login_status: ResMut<LoginStatus>,
    mut login_task: ResMut<LoginTask>,
    tokio_runtime: Res<TokioRuntimeResource>,
) {
    egui::CentralPanel::default().show(contexts.ctx_mut().expect("REASON"), |ui| {
        ui.vertical_centered(|ui| {
            ui.heading("MMORPG Launcher");
            ui.add_space(24.0);

            ui.label("Username");
            ui.text_edit_singleline(&mut login_form.username);

            ui.add_space(8.0);

            ui.label("Password");
            ui.add(
                egui::TextEdit::singleline(&mut login_form.password)
                    .password(true),
            );

            ui.add_space(16.0);

            let is_logging_in = matches!(*login_status, LoginStatus::LoggingIn);

            if ui
                .add_enabled(!is_logging_in, egui::Button::new("Login"))
                .clicked()
            {
                start_login(
                    &login_form.username,
                    &login_form.password,
                    &mut login_status,
                    &mut login_task,
                    &tokio_runtime,
                );
            }

            ui.add_space(16.0);

            draw_login_status(ui, &login_status);
        });
    });
}

fn start_login(
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

fn draw_login_status(ui: &mut egui::Ui, login_status: &LoginStatus) {
    match login_status {
        LoginStatus::Idle => {
            ui.label("Enter your credentials to log in.");
        }
        LoginStatus::LoggingIn => {
            ui.spinner();
            ui.label("Contacting GateKeeper...");
        }
        LoginStatus::Success {
            session_token,
            game_server_address,
        } => {
            ui.colored_label(egui::Color32::GREEN, "Login accepted.");
            ui.label(format!("Session token: {session_token}"));
            ui.label(format!("Game server: {game_server_address}"));
        }
        LoginStatus::Failed { reason } => {
            ui.colored_label(
                egui::Color32::RED,
                format!("Login failed: {reason}"),
            );
        }
        LoginStatus::Error { message } => {
            ui.colored_label(egui::Color32::RED, message);
        }
    }
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
                Ok(LoginResponse::Success {
                       session_token,
                       game_server_address,
                   }) => {
                    *login_status = LoginStatus::Success {
                        session_token,
                        game_server_address,
                    };
                }
                Ok(LoginResponse::Failed { reason }) => {
                    *login_status = LoginStatus::Failed { reason };
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