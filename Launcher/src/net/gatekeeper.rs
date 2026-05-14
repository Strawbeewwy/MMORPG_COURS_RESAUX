use anyhow::{Context, Result};
use shared::protocol::{LoginHttpRequest, LoginHttpResponse};
use shared::config::GATEKEEPER_HTTP_URL;

pub async fn login_to_gatekeeper(
    username: &str,
    password: &str,
) -> Result<LoginHttpResponse> {
    let gatekeeper_url = std::env::var("GATEKEEPER_HTTP_URL")
        .unwrap_or_else(|_| GATEKEEPER_HTTP_URL.to_string());

    let login_url = format!("{gatekeeper_url}/login");

    let request = LoginHttpRequest {
        username: username.to_string(),
        password: password.to_string(),
    };

    let response = reqwest::Client::new()
        .post(&login_url)
        .json(&request)
        .send()
        .await
        .context("failed to send login request to GateKeeper")?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response
            .text()
            .await
            .unwrap_or_else(|_| "failed to read error response body".to_string());

        anyhow::bail!("GateKeeper login failed with status {status}: {error_body}");
    }

    response
        .json::<LoginHttpResponse>()
        .await
        .context("failed to parse GateKeeper login response")
}