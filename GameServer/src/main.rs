use bevy::prelude::*;
use bevy::app::ScheduleRunnerPlugin;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_millis(16), )))
        .run();
}