//! WaOS port: what of a request differs by the system that sends it.
//!
//! On a desktop reqwest's client is hyper's, which lives on tokio's runtime: a send is handed to
//! that runtime from whichever executor asks for it, and the client goes through the proxy the
//! app names.

use reqwest::{ClientBuilder, Proxy, RequestBuilder, Response};
use url::Url;

/// `builder`, with every request going through `proxy`.
pub(super) fn through_proxy(builder: ClientBuilder, proxy: Url) -> ClientBuilder {
    match Proxy::all(proxy.clone()) {
        Ok(proxy) => builder.proxy(proxy),
        Err(e) => {
            tracing::error!("Couldn't configure proxy {proxy}: {e}");
            builder
        }
    }
}

/// Sends `request`, and answers the response once its head is there.
pub(super) async fn send(request: RequestBuilder) -> reqwest::Result<Response> {
    super::spawn_tokio(request.send()).await
}
