//! Model-fetch commands (`src/lib/api/model-fetch.ts`).
//!
//! These commands hit a provider's network endpoint to list available models,
//! so the backing domain fns are async (reqwest). We bridge the synchronous
//! dispatch into the running web runtime with [`super::common::block_on`].
//!
//! Follows the [`super::providers`] template.

use serde_json::{json, Value};

use super::common::{block_on, opt_str_arg, str_arg, to_value};
use crate::services::CodexOAuthService;
use crate::web::error::WebError;
use crate::{AppState, ProviderService};

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

        // OpenAI-compatible GET /v1/models fetch. ProviderService::fetch_provider_models
        // is async -> Result<Vec<String>> (model ids). The extra TS args
        // (isFullUrl/modelsUrl/customUserAgent) aren't supported by the CLI impl
        // and are ignored. Reshape ids into the TS FetchedModel[] contract.
        "fetch_models_for_config" => match str_arg(args, "baseUrl") {
            Ok(base_url) => block_on(ProviderService::fetch_provider_models(
                base_url,
                opt_str_arg(args, "apiKey"),
            ))
            .map_err(WebError::Domain)
            .map(|ids| {
                Value::Array(
                    ids.into_iter()
                        .map(|id| json!({ "id": id, "ownedBy": Value::Null }))
                        .collect(),
                )
            }),
            Err(e) => Err(e),
        },

        _ => return None,
    })
}
