//! Supplementary pricing / quota commands.
//!
//! Follows the [`super::subscription`] / [`super::model_fetch`] templates. Of the
//! four requested commands, only `get_codex_oauth_quota` has a clean cc-switch-cli
//! backing fn; the others fall through to 501 (see SKIP notes below).
//!
//! - `get_model_pricing`        — no clean backing fn (catalog is only read via
//!   TUI-private raw SQL; the DAO exposes upsert/delete/prune only). SKIP.
//! - `stream_check_all_providers` — no batch backing fn; `StreamCheckService`
//!   only exposes single-provider `check_with_retry`. SKIP.
//! - `fetch_models_for_config`  — no domain fn matching its contract
//!   (`baseUrl`/`apiKey`/`isFullUrl`/`modelsUrl`/`customUserAgent` ->
//!   `Vec<FetchedModel>`); `model_fetch` only defines the `FetchedModel` struct.
//!   SKIP.

use serde_json::Value;

use super::common::{block_on, opt_str_arg, to_value};
use crate::services::CodexOAuthService;
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ accountId: string | null }` -> SubscriptionQuota object.
        // CodexOAuthService::get_quota(Option<&str>) -> SubscriptionQuota is async
        // and infallible (it encodes errors as `not_found` / `error` quota
        // variants), so the awaited value is serialized directly.
        "get_codex_oauth_quota" => {
            let account_id = opt_str_arg(args, "accountId");
            to_value(block_on(CodexOAuthService::get_quota(account_id)))
        }

        _ => return None,
    })
}
