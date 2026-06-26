//! Global outbound-proxy commands (`src/lib/api/globalProxy.ts`).
//!
//! Follows the [`super::meta`] template. Only the two commands backed by a
//! cc-switch-cli domain fn are wired:
//!   - `get_global_proxy_url` / `set_global_proxy_url` map to the settings DAO
//!     methods on [`crate::database::Database`] (reachable via `state.db`).
//!
//! The remaining frontend commands (`test_proxy_url`, `get_upstream_proxy_status`,
//! `scan_local_proxies`) have no backing implementation in the CLI and fall
//! through to HTTP 501.

use serde_json::Value;

use super::common::{ok_null, str_arg};
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> string | null (None == direct connection / not configured).
        "get_global_proxy_url" => state
            .db
            .get_global_proxy_url()
            .map(|opt| opt.map(Value::String).unwrap_or(Value::Null))
            .map_err(WebError::Domain),

        // `{ url }` arg -> void. Empty/whitespace clears the proxy (direct), which
        // the DAO handles internally.
        "set_global_proxy_url" => match str_arg(args, "url") {
            Ok(url) => state
                .db
                .set_global_proxy_url(Some(url))
                .map_err(WebError::Domain)
                .and_then(|_| ok_null()),
            Err(e) => Err(e),
        },

        // test_proxy_url, get_upstream_proxy_status, scan_local_proxies have no
        // backing CLI fn -> fall through to 501.
        _ => return None,
    })
}
