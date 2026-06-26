//! Shared helpers for the web command handlers.
//!
//! Every handler module under [`crate::web::handlers`] uses these to parse the
//! `invoke()` args object and shape the JSON response the frontend expects.

use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::web::error::WebError;
use crate::{AppError, AppType};

/// Parse an [`AppType`] from `args[key]`. The frontend passes `{ app: "claude" }`
/// for most commands and `{ appType: "claude" }` for a few config ones.
pub fn app_arg(args: &Value, key: &str) -> Result<AppType, WebError> {
    let raw = args
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| WebError::BadRequest(format!("missing '{key}' argument")))?;
    AppType::from_str(raw).map_err(WebError::Domain)
}

/// Shortcut for the common `{ app: ... }` argument.
pub fn app(args: &Value) -> Result<AppType, WebError> {
    app_arg(args, "app")
}

/// Required string argument.
pub fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, WebError> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| WebError::BadRequest(format!("missing '{key}' argument")))
}

/// Optional string argument (None when absent, null, or not a string).
pub fn opt_str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

/// Optional bool argument, defaulting to `default` when absent.
pub fn bool_arg(args: &Value, key: &str, default: bool) -> bool {
    args.get(key).and_then(Value::as_bool).unwrap_or(default)
}

/// Deserialize a required structured argument (e.g. a `Provider`) from
/// `args[key]`.
pub fn from_arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<T, WebError> {
    let raw = args
        .get(key)
        .ok_or_else(|| WebError::BadRequest(format!("missing '{key}' argument")))?;
    serde_json::from_value(raw.clone())
        .map_err(|e| WebError::BadRequest(format!("invalid '{key}' argument: {e}")))
}

/// Deserialize an optional structured argument; None when the key is absent or
/// JSON `null`.
pub fn opt_from_arg<T: DeserializeOwned>(args: &Value, key: &str) -> Result<Option<T>, WebError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(raw) => serde_json::from_value(raw.clone())
            .map(Some)
            .map_err(|e| WebError::BadRequest(format!("invalid '{key}' argument: {e}"))),
    }
}

/// Serialize a domain value into the JSON the frontend expects.
pub fn to_value<T: Serialize>(value: T) -> Result<Value, WebError> {
    serde_json::to_value(value)
        .map_err(|e| WebError::Domain(AppError::Message(format!("serialization error: {e}"))))
}

/// `Ok(true)` — for commands whose TS signature is `Promise<boolean>` success.
pub fn ok_true() -> Result<Value, WebError> {
    Ok(Value::Bool(true))
}

/// `Ok(null)` — for commands whose TS signature is `Promise<void>`.
pub fn ok_null() -> Result<Value, WebError> {
    Ok(Value::Null)
}

/// Run an async domain fn from a synchronous handler.
///
/// The handler dispatch is synchronous, but some cc-switch-cli services are
/// async (proxy control, WebDAV sync, model fetch/test, quota). We are already
/// inside the multi-threaded web runtime, so `block_in_place` + the current
/// runtime handle lets us await without spawning a nested runtime.
///
/// Requires a multi-threaded runtime (the web server uses one). Tests that hit
/// an async command must use `#[tokio::test(flavor = "multi_thread")]`.
pub fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(fut))
}
