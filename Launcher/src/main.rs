/**
main.rs is the entry point of the application
and needs to stay as minimal as possible.
It only initializes the application and imports
all other modules that are needed.

We could add a Logo here when the launcher
is started.
**/


mod app;
mod config;
mod login;
mod protocol;

fn main() {
    app::run();
}