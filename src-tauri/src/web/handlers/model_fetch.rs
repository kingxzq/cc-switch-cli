//! Model-fetch commands (`src/lib/api/model-fetch.ts`).
//!
//! These commands hit a provider's network endpoint to list available models,
//! so the backing domain fns are async (reqwest). We bridge the synchronous
//! dispatch into the running web runtime with [`super::common::block_on`].
//!
//! Follows the [`super::providers`] template.

use serde_json::Value;

use super::common::{block_on, opt_str_arg, to_value};
use crate::services::CodexOAuthService;
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // Codex OAuth (ChatGPT Plus/Pro) model list. Async; `accountId` is
        // optional (null -> default account resolved inside the service).
        // CodexOAuthService::get_models returns Result<Vec<FetchedModel>, String>.
        "get_codex_oauth_models" => {
            let account_id = opt_str_arg(args, "accountId");
            block_on(CodexOAuthService::get_models(account_id))
                .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                .and_then(to_value)
        }

        // `fetch_models_for_config` is intentionally NOT wired here: there is no
        // cc-switch-cli domain fn matching its contract (baseUrl/apiKey/isFullUrl/
        // modelsUrl/customUserAgent -> Vec<FetchedModel>). It falls through to 501.
        _ => return None,
    })
}
