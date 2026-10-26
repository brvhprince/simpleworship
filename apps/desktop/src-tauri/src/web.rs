//! Requests to the internet over HTTPS: downloads of speech models and
//! Bibles. (Other SimpleWorship computers on the church network are
//! reached with `plain_http` instead.)

use std::time::Duration;

/// How long connecting to a server may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// How long to wait for each piece of an answer. A large download that
/// keeps arriving is never cut off; one that stalls fails instead of
/// waiting for ever.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// An HTTPS client that says it is SimpleWorship, and gives up on a server
/// that does not answer or stops sending.
pub fn client() -> Result<reqwest::Client, reqwest::Error> {
    // The HTTPS library leaves the choice of encryption code to the app
    // (as the updater does too).
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent(concat!("SimpleWorship/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()
}
