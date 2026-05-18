use anyhow::{Context, Result};
use shared::config::GATEKEEPER_HTTP_URL;
use shared::protocol::{LoginHttpRequest, LoginHttpResponse};
use shared::protocol::transport::codec;

pub async fn login_to_gatekeeper(username: &str, password: &str) -> Result<LoginHttpResponse> {
    //get the gatekeeper url
    let gatekeeper_url =
        std::env::var("GATEKEEPER_HTTP_URL").unwrap_or_else(|_| GATEKEEPER_HTTP_URL.to_string());

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

    //read the response body
    let bytes = response
        .bytes()
        .await
        .context("failed to read GateKeeper login response body")?;

    //decode the response body
    codec::decode::<LoginHttpResponse>(&bytes)
        .context("failed to decode GateKeeper login response")
}
