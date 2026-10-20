//! A small HTTP/1.1 client for talking to another SimpleWorship computer's
//! server (`sw-server`) on the local network: plain HTTP only, one request
//! per connection.
//!
//! A general HTTP library is not needed for this, and would bring TLS set-up
//! with it: the one the app already has is built (for the updater) to
//! expect the app to choose a TLS provider first, and creating a client
//! without one crashes.

use std::io;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// An answer from the server.
#[derive(Debug)]
pub struct Response {
    /// The HTTP status, e.g. 200.
    pub status: u16,
    /// The body.
    pub body: Vec<u8>,
}

impl Response {
    /// Whether the status is a success (2xx).
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// The body as text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The body as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_slice(&self.body)
    }
}

/// `GET {address}{path}`, e.g. `get("http://192.168.1.20:7270", "/api/hello", …)`.
pub async fn get(address: &str, path: &str, timeout: Duration) -> io::Result<Response> {
    send(address, "GET", path, None, timeout).await
}

/// `POST {address}{path}` with `body` as JSON.
pub async fn post_json(
    address: &str,
    path: &str,
    body: &impl Serialize,
    timeout: Duration,
) -> io::Result<Response> {
    let body = serde_json::to_vec(body).map_err(io::Error::other)?;
    send(address, "POST", path, Some(body), timeout).await
}

async fn send(
    address: &str,
    method: &str,
    path: &str,
    body: Option<Vec<u8>>,
    timeout: Duration,
) -> io::Result<Response> {
    let host = address
        .strip_prefix("http://")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "only http:// addresses"))?
        .trim_end_matches('/');
    let exchange = async {
        let mut stream = TcpStream::connect(host).await?;
        let mut request =
            format!("{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n");
        if let Some(body) = &body {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).await?;
        if let Some(body) = &body {
            stream.write_all(body).await?;
        }
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await?;
        parse(&raw)
    };
    tokio::time::timeout(timeout, exchange)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "no answer in time"))?
}

/// Splits a whole HTTP/1.1 response into its status and body.
fn parse(raw: &[u8]) -> io::Result<Response> {
    let bad = |what: &str| io::Error::new(io::ErrorKind::InvalidData, what.to_owned());
    let split = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| bad("no end to the headers"))?;
    let head = String::from_utf8_lossy(&raw[..split]);
    let body = &raw[split + 4..];
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| bad("no status"))?;
    let chunked = head.lines().any(|line| {
        let line = line.to_ascii_lowercase();
        line.starts_with("transfer-encoding:") && line.contains("chunked")
    });
    let body = if chunked {
        unchunk(body)?
    } else {
        body.to_vec()
    };
    Ok(Response { status, body })
}

/// Joins a body sent in chunks ("Transfer-Encoding: chunked").
fn unchunk(mut rest: &[u8]) -> io::Result<Vec<u8>> {
    let bad = || io::Error::new(io::ErrorKind::InvalidData, "broken chunks");
    let mut body = Vec::new();
    loop {
        let end = rest.windows(2).position(|w| w == b"\r\n").ok_or_else(bad)?;
        let size_line = String::from_utf8_lossy(&rest[..end]);
        let size = usize::from_str_radix(size_line.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| bad())?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Ok(body);
        }
        if rest.len() < size + 2 {
            return Err(bad());
        }
        body.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    /// A one-request server that answers with `response`, and hands back
    /// what it was sent.
    async fn serve_once(response: &'static [u8]) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0u8; 4096];
            let read = stream.read(&mut request).await.unwrap();
            stream.write_all(response).await.unwrap();
            request.truncate(read);
            request
        });
        (address, task)
    }

    #[tokio::test]
    async fn gets_a_json_answer() {
        let (address, task) =
            serve_once(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"now\": 42}").await;
        let response = get(&address, "/api/time", Duration::from_secs(5))
            .await
            .unwrap();
        assert!(response.ok());
        let value: serde_json::Value = response.json().unwrap();
        assert_eq!(value["now"], 42);
        let request = String::from_utf8(task.await.unwrap()).unwrap();
        assert!(request.starts_with("GET /api/time HTTP/1.1\r\n"));
    }

    #[tokio::test]
    async fn posts_json_and_reads_a_refusal() {
        let (address, task) =
            serve_once(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 5\r\n\r\nWrong").await;
        let response = post_json(
            &address,
            "/api/pair",
            &serde_json::json!({ "code": "1" }),
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        assert_eq!(response.status, 401);
        assert_eq!(response.text(), "Wrong");
        let request = String::from_utf8(task.await.unwrap()).unwrap();
        assert!(request.contains("Content-Type: application/json"));
        assert!(request.ends_with("{\"code\":\"1\"}"));
    }

    #[test]
    fn joins_chunked_bodies() {
        let response = parse(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nHello\r\n6\r\n World\r\n0\r\n\r\n",
        )
        .unwrap();
        assert_eq!(response.text(), "Hello World");
    }

    #[tokio::test]
    async fn gives_up_on_a_silent_server() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let result = get(&address, "/", Duration::from_millis(200)).await;
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
        drop(listener);
    }
}
