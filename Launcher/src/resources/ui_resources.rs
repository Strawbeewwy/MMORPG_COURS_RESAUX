use bevy::prelude::*;

#[derive(Resource, Default)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveLoginField {
    #[default]
    Username,
    Password,
}

pub struct UIResourcesPlugin;
impl Plugin for UIResourcesPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<LoginForm>()
            .init_resource::<ActiveLoginField>();
    }
}
