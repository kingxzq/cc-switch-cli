//! Settings / sync / tool / config-override commands (`src/lib/api/settings.ts`).
//!
//! Follows the [`super::meta`] template. Only commands with a confirmed
//! cc-switch-cli backing fn are wired here. Desktop-only commands (native
//! dialogs, shell/window/update, tray) and commands with no backing
//! implementation (S3 sync, tool versions/lifecycle/probe, db-backup file
//! management, app-config-dir override, WebDAV settings save/test/remote-info)
//! fall through to HTTP 501.

use std::path::PathBuf;

use serde_json::{json, Value};

use super::common::{block_on, from_arg, str_arg, to_value};
use crate::web::error::WebError;
use crate::{AppState, ConfigService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    // WebDAV sync (webdav_sync_upload/download) is owned by the `config` module,
    // which is earlier in the dispatch chain and also syncs to live on download.
    Some(match command {
        // ─── Config export / import ──────────────────────────────
        // `crate::export_config_to_file` is async and mirrors the desktop
        // command exactly, returning the `ConfigTransferResult` JSON.
        "export_config_to_file" => match str_arg(args, "filePath") {
            Ok(file_path) => block_on(crate::export_config_to_file(file_path.to_string()))
                .map_err(|e| WebError::Domain(crate::AppError::Message(e))),
            Err(e) => Err(e),
        },

        // Imports a SQL backup; returns the pre-import backup id, shaped into
        // the `ConfigTransferResult` the frontend expects.
        "import_config_from_file" => match str_arg(args, "filePath") {
            Ok(file_path) => {
                ConfigService::import_config_from_path(&PathBuf::from(file_path), state)
                    .map(|backup_id| {
                        json!({
                            "success": true,
                            "message": "SQL imported successfully",
                            "backupId": backup_id,
                        })
                    })
                    .map_err(WebError::Domain)
            }
            Err(e) => Err(e),
        },

        // Pushes the current providers to their live config files.
        "sync_current_providers_live" => crate::ProviderService::sync_current_to_live(state)
            .map(|_| json!({ "success": true, "message": "Live configuration synchronized" }))
            .map_err(WebError::Domain),

        // ─── Claude plugin / onboarding ──────────────────────────
        // `{ official }` -> bool. official => clear, otherwise => write.
        "apply_claude_plugin_config" => {
            let official = super::common::bool_arg(args, "official", false);
            let result = if official {
                crate::claude_plugin::clear_claude_config()
            } else {
                crate::claude_plugin::write_claude_config()
            };
            result.map(Value::Bool).map_err(WebError::Domain)
        }

        "apply_claude_onboarding_skip" => crate::claude_mcp::set_has_completed_onboarding()
            .map(Value::Bool)
            .map_err(WebError::Domain),

        "clear_claude_onboarding_skip" => crate::claude_mcp::clear_has_completed_onboarding()
            .map(Value::Bool)
            .map_err(WebError::Domain),

        // ─── Optimizer / rectifier / log config (settings DAO) ───
        "get_rectifier_config" => state
            .db
            .get_rectifier_config()
            .map_err(WebError::Domain)
            .and_then(to_value),

        "set_rectifier_config" => match from_arg(args, "config") {
            Ok(config) => state
                .db
                .set_rectifier_config(&config)
                .map(|_| Value::Bool(true))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        "get_optimizer_config" => state
            .db
            .get_optimizer_config()
            .map_err(WebError::Domain)
            .and_then(to_value),

        "set_optimizer_config" => match from_arg(args, "config") {
            Ok(config) => state
                .db
                .set_optimizer_config(&config)
                .map(|_| Value::Bool(true))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        "get_log_config" => state
            .db
            .get_log_config()
            .map_err(WebError::Domain)
            .and_then(to_value),

        "set_log_config" => match from_arg(args, "config") {
            Ok(config) => state
                .db
                .set_log_config(&config)
                .map(|_| Value::Bool(true))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        _ => return None,
    })
}
