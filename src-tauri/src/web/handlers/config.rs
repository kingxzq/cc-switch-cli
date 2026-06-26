//! Config commands (`src/lib/api/config.ts` + the backup/WebDAV slice of
//! `src/lib/api/settings.ts`).
//!
//! Focus: common-config snippet set/extract/clear, plus WebDAV sync. Follows the
//! [`super::providers`] template. `get_common_config_snippet` is already wired in
//! [`super::meta`] and is intentionally not handled here.
//!
//! Commands left unwired (no clean cc-switch-cli backing fn / shape divergence)
//! fall through to HTTP 501 and are recorded in the module summary.

use serde_json::{json, Value};

use super::common::{app_arg, opt_str_arg, str_arg};
use crate::web::error::WebError;
use crate::{AppState, AppType, ProviderService, WebDavSyncService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // ── Common config snippet ──────────────────────────────────────────

        // Deprecated Claude-only getter -> string | null. Reads the in-memory
        // snapshot, mirroring how `get_common_config_snippet` is served in `meta`.
        "get_claude_common_config_snippet" => match state.config.read() {
            Ok(cfg) => Ok(cfg
                .common_config_snippets
                .get(&AppType::Claude)
                .map(|s| Value::String(s.clone()))
                .unwrap_or(Value::Null)),
            Err(e) => Err(WebError::Domain(crate::AppError::from(e))),
        },

        // Deprecated Claude-only setter -> void. Validation happens inside the
        // service (`validate_common_config_snippet`).
        "set_claude_common_config_snippet" => match str_arg(args, "snippet") {
            Ok(snippet) => ProviderService::set_common_config_snippet(
                state,
                AppType::Claude,
                Some(snippet.to_string()),
            )
            .map(|_| Value::Null)
            .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ appType, snippet }` -> void. The frontend always passes a string
        // (JSON for Claude/Gemini, TOML for Codex); the service validates it.
        "set_common_config_snippet" => match (app_arg(args, "appType"), str_arg(args, "snippet")) {
            (Ok(app_type), Ok(snippet)) => ProviderService::set_common_config_snippet(
                state,
                app_type,
                Some(snippet.to_string()),
            )
            .map(|_| Value::Null)
            .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ appType, settingsConfig? }` -> string. When `settingsConfig` is
        // present, extract from it; otherwise extract from the current provider.
        "extract_common_config_snippet" => match app_arg(args, "appType") {
            Ok(app_type) => match opt_str_arg(args, "settingsConfig") {
                Some(raw) => match serde_json::from_str::<Value>(raw) {
                    Ok(settings_config) => {
                        ProviderService::extract_common_config_snippet_from_settings(
                            app_type,
                            &settings_config,
                        )
                        .map(Value::String)
                        .map_err(WebError::Domain)
                    }
                    Err(e) => Err(WebError::BadRequest(format!(
                        "invalid 'settingsConfig' argument: {e}"
                    ))),
                },
                None => ProviderService::extract_common_config_snippet(state, app_type)
                    .map(Value::String)
                    .map_err(WebError::Domain),
            },
            Err(e) => Err(e),
        },

        // ── WebDAV sync ────────────────────────────────────────────────────
        // Both operate on the stored WebDAV settings (no args). The service
        // wrappers are synchronous (they drive their own HTTP runtime), so no
        // `block_on` is needed. TS expects `WebDavSyncResult = { status: string }`.
        "webdav_sync_upload" => WebDavSyncService::upload()
            .map(|summary| json!({ "status": summary.message }))
            .map_err(WebError::Domain),

        "webdav_sync_download" => match WebDavSyncService::download() {
            Ok(summary) => {
                // Best-effort post-download live-config sync, matching the CLI flow.
                if let Err(err) = ProviderService::sync_current_to_live(state) {
                    log::warn!("Live config sync after WebDAV download failed: {err}");
                }
                Ok(json!({ "status": summary.message }))
            }
            Err(e) => Err(WebError::Domain(e)),
        },

        // Backups (create/list/restore/rename/delete) and the remaining WebDAV
        // commands (test_connection, save_settings, fetch_remote_info) are not
        // wired: cc-switch-cli has no matching `Database`/service fns with the
        // shapes the frontend expects. They fall through to 501.
        _ => return None,
    })
}
