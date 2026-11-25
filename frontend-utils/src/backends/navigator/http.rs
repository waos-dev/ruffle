//! WaOS port: what of a request differs by the system that sends it.
//!
//! On a desktop reqwest's client is hyper's, which lives on tokio's runtime: a send is handed to
//! that runtime from whichever executor asks for it, and the client goes through the proxy the
//! app names. On WaOS the client is WaOS's own, through reqwest's fork: a send is awaited where
//! it is asked for, and every request goes through the system's proxy, which is WaOS's to choose,
//! so a proxy the app names cannot be used.

use reqwest::{ClientBuilder, RequestBuilder, Response};
use url::Url;

/// `builder`, with every request going through `proxy`.
#[cfg(not(target_os = "wasi"))]
pub(super) fn through_proxy(builder: ClientBuilder, proxy: Url) -> ClientBuilder {
    match reqwest::Proxy::all(proxy.clone()) {
        Ok(proxy) => builder.proxy(proxy),
        Err(e) => {
            tracing::error!("Couldn't configure proxy {proxy}: {e}");
            builder
        }
    }
}

/// `builder` as it is, saying that the proxy asked for is not used.
#[cfg(target_os = "wasi")]
pub(super) fn through_proxy(builder: ClientBuilder, _proxy: Url) -> ClientBuilder {
    tracing::warn!(
        "The proxy asked for is not used: on WaOS requests go through the system's proxy"
    );
    builder
}

/// Sends `request`, and answers the response once its head is there.
pub(super) async fn send(request: RequestBuilder) -> reqwest::Result<Response> {
    #[cfg(not(target_os = "wasi"))]
    return super::spawn_tokio(request.send()).await;
    #[cfg(target_os = "wasi")]
    return request.send().await;
}
