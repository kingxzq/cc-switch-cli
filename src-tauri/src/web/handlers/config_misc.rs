//! Supplementary config / settings commands (`src/lib/api/settings.ts`,
//! `src/lib/api/deeplink.ts`).
//!
//! Follows the [`super::meta`] template. Only commands with a confirmed
//! cc-switch-cli backing fn are wired here. The remaining commands in this group
//! have no faithful backing implementation in the CLI crate and fall through to
//! HTTP 501:
//!
//!   - `get_app_config_dir_override` / `set_app_config_dir_override`: the desktop
//!     reads/writes a Tauri Store via `app_store`. The CLI has no `app_store`
//!     module — `config::get_app_config_dir` explicitly documents "CLI mode: no
//!     app store override, always use default", so there is nothing to read or
//!     persist.
//!   - `webdav_test_connection` / `webdav_sync_save_settings` /
//!     `webdav_sync_fetch_remote_info`: the desktop commands take a `settings`
//!     payload, resolve the password against the stored credentials
//!     (`preserveEmptyPassword` / `passwordTouched`), and return
//!     `WebDavTestResult` / `{ success }` / `RemoteSnapshotInfo`. The CLI
//!     `WebDavSyncService` only exposes `check_connection` / `upload` /
//!     `download` / `migrate_v1_to_v2`, none of which accept a settings payload,
//!     apply the password-preservation logic, or return remote snapshot info.
//!     Wiring them via `settings::set_webdav_sync_settings` would drop that logic
//!     and could overwrite the stored password, so they are skipped.
//!   - `merge_deeplink_config`: the only backing fn
//!     (`deeplink::provider::parse_and_merge_config`) lives in the private
//!     `deeplink` module and is not re-exported, so it cannot be called here.

use serde_json::Value;

use crate::web::error::WebError;
use crate::{config, AppState};

pub fn dispatch(
    _state: &AppState,
    command: &str,
    _args: &Value,
) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> string. Mirrors the desktop command, which returns
        // `get_claude_settings_path()` as a lossy string.
        "get_claude_code_config_path" => Ok(Value::String(
            config::get_claude_settings_path()
                .to_string_lossy()
                .into_owned(),
        )),

        _ => return None,
    })
}
