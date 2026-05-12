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
                padding: UiRect::all(Val::Px(28.0)),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Stretch,
                ..default()
            },
            BackgroundColor(Color::srgb(0.025, 0.022, 0.02)),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(72.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
                .with_children(|header| {
                    header.spawn((
                        Text::new("MMORPG"),
                        TextFont {
                            font_size: 42.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.95, 0.82, 0.48)),
                    ));

                    header.spawn((
                        Text::new("Launcher"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.72, 0.68, 0.58)),
                    ));
                });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    column_gap: Val::Px(28.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Stretch,
                    ..default()
                },
            ))
                .with_children(|content| {
                    content.spawn((
                        Node {
                            flex_grow: 1.0,
                            padding: UiRect::all(Val::Px(30.0)),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexEnd,
                            align_items: AlignItems::FlexStart,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.075, 0.055, 0.045)),
                        BorderColor::from(Color::srgba(0.95, 0.62, 0.22, 0.22)),
                    ))
                        .with_children(|hero| {
                            hero.spawn((
                                Text::new("Welcome back, adventurer"),
                                TextFont {
                                    font_size: 34.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.96, 0.9, 0.78)),
                            ));

                            hero.spawn((
                                Text::new("Prepare your account, enter the realm, and continue your journey."),
                                TextFont {
                                    font_size: 17.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.68, 0.64, 0.56)),
                            ));

                            hero.spawn((
                                Node {
                                    height: Val::Px(18.0),
                                    ..default()
                                },
                            ));

                            hero.spawn((
                                Text::new("NEWS"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.68, 0.32)),
                            ));

                            hero.spawn((
                                Text::new("• GateKeeper authentication online\n• Network systems enabled\n• Game server routing available"),
                                TextFont {
                                    font_size: 15.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.78, 0.75, 0.68)),
                            ));
                        });

                    content.spawn((
                        Node {
                            width: Val::Px(390.0),
                            padding: UiRect::all(Val::Px(26.0)),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(13.0),
                            justify_content: JustifyContent::FlexStart,
                            align_items: AlignItems::Stretch,
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.055, 0.048, 0.043, 0.96)),
                        BorderColor::from(Color::srgba(0.95, 0.62, 0.22, 0.32)),
                    ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new("Account Login"),
                                TextFont {
                                    font_size: 28.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.86, 0.66)),
                            ));

                            panel.spawn((
                                Text::new("Sign in to connect to the realm."),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.62, 0.58, 0.52)),
                            ));

                            panel.spawn((
                                Node {
                                    height: Val::Px(8.0),
                                    ..default()
                                },
                            ));

                            spawn_label(panel, "ACCOUNT NAME");
                            spawn_input_field(panel, UsernameFieldButton, UsernameFieldText, "");

                            spawn_label(panel, "PASSWORD");
                            spawn_input_field(panel, PasswordFieldButton, PasswordFieldText, "");

                            panel
                                .spawn((
                                    Button,
                                    LoginButton,
                                    Node {
                                        height: Val::Px(48.0),
                                        margin: UiRect::top(Val::Px(10.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border: UiRect::all(Val::Px(1.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.58, 0.29, 0.09)),
                                    BorderColor::from(Color::srgb(0.98, 0.65, 0.28)),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("LOG IN"),
                                        TextFont {
                                            font_size: 19.0,
                                            ..default()
                                        },
                                        TextColor(Color::srgb(1.0, 0.92, 0.78)),
                                    ));
                                });

                            panel.spawn((
                                Text::new("Enter your credentials to log in."),
                                LoginStatusText,
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.76, 0.72, 0.65)),
                            ));
                        });
                });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(34.0),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
                .with_children(|footer| {
                    footer.spawn((
                        Text::new("Version 0.1.0"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.48, 0.45, 0.4)),
                    ));

                    footer.spawn((
                        Text::new("GateKeeper Network"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.48, 0.45, 0.4)),
                    ));
                });
        });
}

fn spawn_label(parent: &mut ChildSpawnerCommands, label: &str) {
    parent.spawn((
        Text::new(label),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.62, 0.34)),
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
                height: Val::Px(44.0),
                padding: UiRect::horizontal(Val::Px(12.0)),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.032, 0.03, 0.028)),
            BorderColor::from(Color::srgb(0.28, 0.22, 0.16)),
        ))
        .with_children(|field| {
            field.spawn((
                Text::new(initial_text),
                text_marker,
                TextFont {
                    font_size: 17.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.88, 0.78)),
            ));
        });
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
            Color::srgb(0.18, 0.16, 0.14)
        } else {
            match *interaction {
                Interaction::Pressed => Color::srgb(0.42, 0.18, 0.05),
                Interaction::Hovered => Color::srgb(0.78, 0.39, 0.12),
                Interaction::None => Color::srgb(0.58, 0.29, 0.09),
            }
        };
    }
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

fn login_status_message_and_color(login_status: &LoginStatus) -> (String, Color) {
    match login_status {
        LoginStatus::Idle => (
            "Enter your credentials to log in.".to_string(),
            Color::srgb(0.76, 0.72, 0.65),
        ),
        LoginStatus::LoggingIn => (
            "Contacting GateKeeper...".to_string(),
            Color::srgb(0.95, 0.72, 0.38),
        ),
        LoginStatus::Success {
            session_token,
            game_server_address,
        } => (
            format!(
                "Login accepted.\nSession token: {session_token}\nGame server: {game_server_address}"
            ),
            Color::srgb(0.42, 1.0, 0.55),
        ),
        LoginStatus::Failed { reason } => (
            format!("Login failed: {reason}"),
            Color::srgb(1.0, 0.34, 0.24),
        ),
        LoginStatus::Error { message } => (
            message.clone(),
            Color::srgb(1.0, 0.34, 0.24),
        ),
    }
}