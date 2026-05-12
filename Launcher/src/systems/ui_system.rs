/**
ui_system contains the systems that are used to draw the
login UI.
**/

use bevy::prelude::*;

use bevy::input::keyboard::KeyboardInput;


use crate::resources::ui_resources::{
    LoginForm, ActiveLoginField,
};

use crate::resources::network_resources::{LoginRequestMessage, LoginStatus};


#[derive(Component)]
struct UsernameFieldText;

#[derive(Component)]
struct PasswordFieldText;

#[derive(Component)]
struct LoginStatusText;

#[derive(Component)]
struct LoginButton;

#[derive(Component)]
struct UsernameFieldButton;

#[derive(Component)]
struct PasswordFieldButton;

/**
This plugin adds the systems as update so that they are run
each frame.
**/
pub struct UISystemPlugin;

impl Plugin for UISystemPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_login_ui)
            .add_systems(
                Update,
                (
                    handle_login_field_clicks,
                    handle_login_text_input,
                    handle_login_button,
                    update_login_form_text,
                    update_login_status_text,
                    update_login_button_style,
                ),
            );
    }
}

fn spawn_login_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.03, 0.035, 0.05)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(420.0),
                        padding: UiRect::all(Val::Px(32.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(14.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.08, 0.09, 0.13)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("MMORPG Launcher"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    spawn_label(panel, "Username");
                    spawn_input_field(panel, UsernameFieldButton, UsernameFieldText, "");

                    spawn_label(panel, "Password");
                    spawn_input_field(panel, PasswordFieldButton, PasswordFieldText, "");

                    panel
                        .spawn((
                            Button,
                            LoginButton,
                            Node {
                                height: Val::Px(44.0),
                                margin: UiRect::top(Val::Px(8.0)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.18, 0.35, 0.85)),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Login"),
                                TextFont {
                                    font_size: 20.0,
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        });

                    panel.spawn((
                        Text::new("Enter your credentials to log in."),
                        LoginStatusText,
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.82, 0.9)),
                    ));
                });
        });
}

fn spawn_label(parent: &mut ChildSpawnerCommands, label: &str) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.75, 0.78, 0.9)),
    ));
}

fn spawn_input_field<TButton, TText>(
    parent: &mut ChildSpawnerCommands,
    button_marker: TButton,
    text_marker: TText,
    initial_text: &str,
) where
    TButton: Component,
    TText: Component,
{
    parent
        .spawn((
            Button,
            button_marker,
            Node {
                height: Val::Px(42.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.13, 0.18)),
        ))
        .with_children(|field| {
            field.spawn((
                Text::new(initial_text),
                text_marker,
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn handle_login_field_clicks(
    mut active_field: ResMut<ActiveLoginField>,
    username_query: Query<&Interaction, (Changed<Interaction>, With<UsernameFieldButton>)>,
    password_query: Query<&Interaction, (Changed<Interaction>, With<PasswordFieldButton>)>,
) {
    for interaction in &username_query {
        if *interaction == Interaction::Pressed {
            *active_field = ActiveLoginField::Username;
        }
    }

    for interaction in &password_query {
        if *interaction == Interaction::Pressed {
            *active_field = ActiveLoginField::Password;
        }
    }
}

fn handle_login_text_input(
    mut keyboard_input_events: MessageReader<KeyboardInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    active_field: Res<ActiveLoginField>,
    mut login_form: ResMut<LoginForm>,
) {
    for event in keyboard_input_events.read() {
        if !event.state.is_pressed() {
            continue;
        }

        if keyboard.just_pressed(KeyCode::Tab) {
            *login_form = LoginForm {
                username: login_form.username.clone(),
                password: login_form.password.clone(),
            };
            continue;
        }

        if keyboard.just_pressed(KeyCode::Backspace) {
            match *active_field {
                ActiveLoginField::Username => {
                    login_form.username.pop();
                }
                ActiveLoginField::Password => {
                    login_form.password.pop();
                }
            }
            continue;
        }

        let Some(text) = &event.text else {
            continue;
        };

        if text.chars().any(|character| character.is_control()) {
            continue;
        }

        match *active_field {
            ActiveLoginField::Username => login_form.username.push_str(text),
            ActiveLoginField::Password => login_form.password.push_str(text),
        }
    }
}
fn handle_login_button(
    mut button_query: Query<&Interaction, (Changed<Interaction>, With<LoginButton>)>,
    login_form: Res<LoginForm>,
    mut login_messages: MessageWriter<LoginRequestMessage>,
) {
    for interaction in &mut button_query {
        if *interaction == Interaction::Pressed {
            login_messages.write(LoginRequestMessage {
                username: login_form.username.clone(),
                password: login_form.password.clone(),
            });
        }
    }
}

fn update_login_form_text(
    login_form: Res<LoginForm>,

    mut text_queries: ParamSet<(
        Query<&mut Text, With<UsernameFieldText>>,
        Query<&mut Text, With<PasswordFieldText>>,
    )>,
) {
    if !login_form.is_changed() {
        return;
    }
    for mut text in &mut text_queries.p0() {
        **text = login_form.username.clone();
    }

    let masked_password = "●".repeat(login_form.password.chars().count());

    for mut text in &mut text_queries.p1() {
        **text = masked_password.clone();
    }
}

fn update_login_status_text(
    login_status: Res<LoginStatus>,
    mut query: Query<(&mut Text, &mut TextColor), With<LoginStatusText>>,
) {
    if !login_status.is_changed() {
        return;
    }

    let (message, color) = login_status_message_and_color(&login_status);

    for (mut text, mut text_color) in &mut query {
        **text = message.clone();
        text_color.0 = color;
    }
}

fn update_login_button_style(
    login_status: Res<LoginStatus>,
    mut query: Query<
        (&Interaction, &mut BackgroundColor),
        (With<LoginButton>, Changed<Interaction>),
    >,
) {
    let is_logging_in = matches!(*login_status, LoginStatus::LoggingIn);

    for (interaction, mut background_color) in &mut query {
        background_color.0 = if is_logging_in {
            Color::srgb(0.2, 0.2, 0.25)
        } else {
            match *interaction {
                Interaction::Pressed => Color::srgb(0.12, 0.25, 0.65),
                Interaction::Hovered => Color::srgb(0.24, 0.43, 1.0),
                Interaction::None => Color::srgb(0.18, 0.35, 0.85),
            }
        };
    }
}

fn login_status_message_and_color(login_status: &LoginStatus) -> (String, Color) {
    match login_status {
        LoginStatus::Idle => (
            "Enter your credentials to log in.".to_string(),
            Color::srgb(0.8, 0.82, 0.9),
        ),
        LoginStatus::LoggingIn => (
            "Contacting GateKeeper...".to_string(),
            Color::srgb(0.8, 0.82, 0.9),
        ),
        LoginStatus::Success {
            session_token,
            game_server_address,
        } => (
            format!(
                "Login accepted.\nSession token: {session_token}\nGame server: {game_server_address}"
            ),
            Color::srgb(0.35, 1.0, 0.45),
        ),
        LoginStatus::Failed { reason } => (
            format!("Login failed: {reason}"),
            Color::srgb(1.0, 0.25, 0.25),
        ),
        LoginStatus::Error { message } => (
            message.clone(),
            Color::srgb(1.0, 0.25, 0.25),
        ),
    }
}
