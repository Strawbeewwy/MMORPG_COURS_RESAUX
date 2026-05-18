
use bevy::prelude::*;

pub fn run() {
    tracing_subscriber::fmt::init();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MMORPG Lab - Bevy Game Client".to_string(),
                resolution: (900, 600).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_camera)
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}