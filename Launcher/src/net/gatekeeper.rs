use anyhow::{Context, Result};
use shared::config::GATEKEEPER_HTTP_URL;
use shared::protocol::{LoginHttpRequest, LoginHttpResponse, HealthResponse};


pub async fn login_to_gatekeeper(username: &str, password: &str) -> Result<LoginHttpResponse> {
    //get the gatekeeper url
    let gatekeeper_url =
        std::env::var("GATEKEEPER_HTTP_URL").unwrap_or_else(|_| GATEKEEPER_HTTP_URL.to_string());

    let client = reqwest::Client::new();

    let health_url = format!("{gatekeeper_url}/health");
    let health_response = client
        .get(&health_url)
        .send()
        .await
        .context("failed to reach GateKeeper health endpoint")?;

    if !health_response.status().is_success() {
        let status = health_response.status();
        let error_body = health_response
            .text()
            .await
            .unwrap_or_else(|_| "failed to read health response body".to_string());

        anyhow::bail!("GateKeeper health check failed with status {status}: {error_body}");
    }

    let health = health_response
        .json::<HealthResponse>()
        .await
        .context("failed to decode GateKeeper health response")?;

    tracing::info!("GateKeeper health response: {health:?}");
    

    //build the login url
    let login_url = format!("{gatekeeper_url}/login");

    //build the login request info
    let request = LoginHttpRequest {
        username: username.to_string(),
        password: password.to_string(),
    };

    /*
    send the login request to gatekeeper
    if the request is not acknowledged ie: invalid url, timed out, etc,
    print an error and bail
    */
    let response = client
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
        .context("failed to decode GateKeeper login response")

}
