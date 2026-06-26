//! OMO / omo_slim commands (`src/lib/api/omo.ts`).
//!
//! The frontend exposes two app surfaces here (OMO and omo_slim), each with
//! read_omo[_slim]_local_file, get_current_omo[_slim]_provider_id, and
//! disable_current_omo[_slim].
//!
//! cc-switch-cli's [`crate::AppType`] only supports claude, codex, gemini,
//! opencode, hermes, and openclaw — there is no `omo` / `omo_slim` app type, no
//! `OmoLocalFileData` model, and no backing service for reading the OMO live
//! file, resolving its current provider id, or disabling it. Every command in
//! this module is therefore left unwired and falls through to HTTP 501.
//!
//! This stub exists so the module slot is filled and these commands have a
//! documented home; wire them only after the OMO domain layer lands in
//! cc-switch-cli.

use serde_json::Value;

use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(
    _state: &AppState,
    _command: &str,
    _args: &Value,
) -> Option<Result<Value, WebError>> {
    // No OMO/omo_slim backing fns exist in cc-switch-cli — every command here
    // (read_omo_local_file, get_current_omo_provider_id, disable_current_omo and
    // the omo_slim variants) falls through to HTTP 501. See module docs.
    None
}
