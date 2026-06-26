//! App-level / settings-core commands.
//!
//! This module is the REFERENCE TEMPLATE for the other handler modules — it
//! demonstrates every arg/return pattern in use. A handler module exposes a
//! single `dispatch` fn that returns:
//!   - `Some(Ok(value))`  — command handled, resolves the TS promise with `value`
//!   - `Some(Err(WebError))` — command handled but failed (rejects the promise)
//!   - `None` — command does not belong to this module (the chain tries the next)
//!
//! Only wire a command after reading the cc-switch-cli fn it maps to and
//! confirming its signature. Commands left unwired fall through to HTTP 501.

use std::path::PathBuf;

use serde_json::Value;

use super::common::{app, from_arg, ok_true, to_value};
use crate::web::error::WebError;
use crate::{config, settings, AppState, AppType};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    // `state: &AppState` carries the DB + in-memory config snapshot. This module
    // mostly uses process-global singletons (settings::*, config::*), so it
    // touches `state` only for the common-config snapshot below.
    Some(match command {
        // No args -> object. AppSettings serializes camelCase to match TS Settings.
        "get_settings" => to_value(settings::get_settings()),

        // Deserialize a structured arg -> bool.
        "save_settings" => match from_arg(args, "settings") {
            Ok(parsed) => settings::update_settings(parsed)
                .map(|_| Value::Bool(true))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ app }` arg -> string path.
        "get_config_dir" => match app(args) {
            Ok(app_type) => Ok(Value::String(
                config_dir_for(&app_type).to_string_lossy().into_owned(),
            )),
            Err(e) => Err(e),
        },

        // `{ appType }` arg -> string | null (reads the in-memory snapshot).
        "get_common_config_snippet" => match super::common::app_arg(args, "appType") {
            Ok(app_type) => match state.config.read() {
                Ok(cfg) => Ok(cfg
                    .common_config_snippets
                    .get(&app_type)
                    .map(|s| Value::String(s.clone()))
                    .unwrap_or(Value::Null)),
                Err(e) => Err(WebError::Domain(crate::AppError::from(e))),
            },
            Err(e) => Err(e),
        },

        // No args -> string.
        "get_app_config_path" => Ok(Value::String(
            config::get_app_config_path().to_string_lossy().into_owned(),
        )),

        // No args -> bool.
        "is_portable_mode" => Ok(Value::Bool(false)),

        // Desktop-only no-op kept so the post-switch flow completes.
        "update_tray_menu" => ok_true(),

        // Not a real frontend invoke (the app shim provides the version), but
        // harmless if ever called.
        "get_app_version" => Ok(Value::String(env!("CARGO_PKG_VERSION").to_string())),

        _ => return None,
    })
}

/// The live config directory for an app (shown in the directory-settings panel).
fn config_dir_for(app: &AppType) -> PathBuf {
    match app {
        AppType::Claude => config::get_claude_config_dir(),
        AppType::Codex => crate::codex_config::get_codex_config_dir(),
        AppType::Gemini => crate::gemini_config::get_gemini_dir(),
        AppType::OpenCode => crate::opencode_config::get_opencode_dir(),
        AppType::Hermes => crate::hermes_config::get_hermes_dir(),
        AppType::OpenClaw => crate::openclaw_config::get_openclaw_dir(),
    }
}
