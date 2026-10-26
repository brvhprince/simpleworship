//! Requests to the internet over HTTPS: downloads of speech models and
//! Bibles. (Other SimpleWorship computers on the church network are
//! reached with `plain_http` instead.)

/// An HTTPS client that says it is SimpleWorship.
pub fn client() -> Result<reqwest::Client, reqwest::Error> {
    // The HTTPS library leaves the choice of encryption code to the app
    // (as the updater does too).
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    reqwest::Client::builder()
        .user_agent(concat!("SimpleWorship/", env!("CARGO_PKG_VERSION")))
        .build()
}
