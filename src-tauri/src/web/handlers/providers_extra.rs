//! Extra provider commands from `src/lib/api/providers.ts` not handled by
//! [`super::providers`].
//!
//! Follows the [`super::meta`] template. This module covers the live
//! provider-id helpers for the additive-mode apps (OpenCode / OpenClaw /
//! Hermes). Each maps to the app-specific `*_config::get_providers()` reader and
//! returns the provider id list as a JSON `string[]`.
//!
//! Intentionally NOT wired here (left to fall through to 501):
//!   - Universal provider commands (`get_universal_providers`,
//!     `get_universal_provider`, `upsert_universal_provider`,
//!     `delete_universal_provider`, `sync_universal_provider`): the cc-switch-cli
//!     crate has no `UniversalProvider` type and the orphaned
//!     `database/dao/universal_providers.rs` is not even declared in `dao/mod.rs`,
//!     so there is no compilable backing fn.
//!   - `claude_desktop_*`, `ensure_claude_desktop_official_provider`,
//!     `open_provider_terminal`, `import_claude_desktop_providers_from_claude`:
//!     desktop-only (native shell/Claude Desktop integration), no CLI impl.

use serde_json::Value;

use super::common::to_value;
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(
    _state: &AppState,
    command: &str,
    _args: &Value,
) -> Option<Result<Value, WebError>> {
    Some(match command {
        // Live provider ids in the OpenCode config (`opencode.json`).
        // TS: getOpenCodeLiveProviderIds(): Promise<string[]>
        "get_opencode_live_provider_ids" => crate::opencode_config::get_providers()
            .map(|providers| providers.keys().cloned().collect::<Vec<String>>())
            .map_err(WebError::Domain)
            .and_then(to_value),

        // Live provider ids in the OpenClaw config (`openclaw.json`).
        // TS: getOpenClawLiveProviderIds(): Promise<string[]>
        "get_openclaw_live_provider_ids" => crate::openclaw_config::get_providers()
            .map(|providers| providers.keys().cloned().collect::<Vec<String>>())
            .map_err(WebError::Domain)
            .and_then(to_value),

        // Live provider ids in the Hermes config.
        // TS: getHermesLiveProviderIds(): Promise<string[]>
        "get_hermes_live_provider_ids" => crate::hermes_config::get_providers()
            .map(|providers| providers.keys().cloned().collect::<Vec<String>>())
            .map_err(WebError::Domain)
            .and_then(to_value),

        _ => return None,
    })
}
