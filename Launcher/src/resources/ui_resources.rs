use bevy::prelude::*;

/*
This resource is used to store the username and password
of the user that wants to log in.
*/
#[derive(Resource, Default)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/*
This resource is used to store the currently active login field.
*/
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveLoginField {
    #[default]
    Username,
    Password,
}

pub struct UIResourcesPlugin;
impl Plugin for UIResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginForm>()
            .init_resource::<ActiveLoginField>();
    }
}
