use anyhow::{anyhow, Context, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;


use crate::protocol::codec;

/// Send one serialized message on a QUIC send stream.
///
/// The stream is finished after sending, so the peer can safely use
/// `read_to_end` to know when the message is complete.
pub async fn send_message<T>(
    send_stream: &mut quinn::SendStream,
    message: &T,
) -> Result<()>
where
    T: Serialize,
{
    let payload = codec::encode(message)
        .context("failed to encode QUIC protocol message")?;

    send_stream
        .write_all(&payload)
        .await
        .context("failed to write QUIC protocol message")?;

    send_stream
        .finish()
        .context("failed to finish QUIC send stream")?;

    Ok(())
}

/// Receive one serialized message from a QUIC receive stream.
///
/// The caller provides a maximum size to avoid reading unbounded data.
pub async fn receive_message<T>(
    receive_stream: &mut quinn::RecvStream,
    size_limit: usize,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let payload = receive_stream
        .read_to_end(size_limit)
        .await
        .context("failed to read QUIC protocol message")?;

    if payload.is_empty() {
        return Err(anyhow!("empty QUIC protocol message"));
    }

    codec::decode(&payload)
        .context("failed to decode QUIC protocol message")
}

/// Send a request and receive a response over a new bidirectional QUIC stream.
pub async fn send_request<Request, Response>(
    connection: &quinn::Connection,
    request: &Request,
    response_size_limit: usize,
) -> Result<Response>
where
    Request: Serialize,
    Response: DeserializeOwned,
{
    let (mut send_stream, mut receive_stream) = connection
        .open_bi()
        .await
        .context("failed to open QUIC bidirectional stream")?;

    send_message(&mut send_stream, request).await?;

    receive_message(&mut receive_stream, response_size_limit).await
}

/// Receive a request and send a response on an accepted bidirectional stream.
pub async fn handle_request<Request, Response, Handler>(
    mut send_stream: quinn::SendStream,
    mut receive_stream: quinn::RecvStream,
    request_size_limit: usize,
    handler: Handler,
) -> Result<()>
where
    Request: DeserializeOwned,
    Response: Serialize,
    Handler: FnOnce(Request) -> Response,
{
    let request: Request = receive_message(&mut receive_stream, request_size_limit).await?;
    let response = handler(request);
    send_message(&mut send_stream, &response).await?;

    Ok(())
}