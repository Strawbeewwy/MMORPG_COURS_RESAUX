pub fn create_session_token(username: &str) -> String {
    format!("dev-session-token-for-{username}")
}