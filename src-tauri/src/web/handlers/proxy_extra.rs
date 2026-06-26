//! Supplementary proxy commands not covered by the main proxy handlers.
//!
//! Follows the [`super::meta`] template. Only `stop_proxy_server` has a backing
//! cc-switch-cli domain fn and is wired here:
//!   - `stop_proxy_server` -> [`crate::services::proxy::ProxyService::stop`]
//!     (async, returns `Result<(), String>`). This stops the proxy runtime only,
//!     without restoring other apps' takeover state — distinct from
//!     `stop_proxy_with_restore` handled in [`super::proxy`].
//!
//! The remaining frontend commands (`scan_local_proxies`, `test_proxy_url`,
//! `get_upstream_proxy_status`) have no backing implementation in the CLI and
//! fall through to HTTP 501 (also noted in [`super::global_proxy`]).

use serde_json::Value;

use super::common::{block_on, ok_null};
use crate::web::error::WebError;
use crate::AppState;

/// Map a service-layer `String` error into a domain [`WebError`].
fn domain_str(e: String) -> WebError {
    WebError::Domain(crate::AppError::Message(e))
}

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    let _ = args; // wired commands take no args
    Some(match command {
        // No args -> void. Service fn returns Result<(), String>. Stops the proxy
        // runtime only (does not restore takeover state).
        "stop_proxy_server" => match block_on(async { state.proxy_service.stop().await }) {
            Ok(()) => ok_null(),
            Err(e) => Err(domain_str(e)),
        },

        // scan_local_proxies, test_proxy_url, get_upstream_proxy_status have no
        // backing CLI fn -> fall through to 501.
        _ => return None,
    })
}
