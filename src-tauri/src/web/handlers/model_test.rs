//! Model/provider connectivity-test commands (`src/lib/api/model-test.ts`).
//!
//! Reachability checks only probe a provider's `base_url`; they do not send a
//! real model request. The probe itself is async, so the network commands are
//! wrapped with [`super::common::block_on`].
//!
//! Mirrors `cli/commands/provider_inspect::stream_check_provider`, but resolves
//! the provider from the supplied `state` (the web runtime owns it) instead of
//! constructing fresh startup state.

use serde_json::Value;

use super::common::{app_arg, from_arg, str_arg, to_value};
use crate::services::stream_check::{StreamCheckConfig, StreamCheckService};
use crate::web::error::WebError;
use crate::{AppError, AppState, ProviderService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ appType, providerId }` -> StreamCheckResult object.
        // Async probe wrapped with block_on. Resolves the provider and the
        // global check config from `state`, then runs check_with_retry.
        "stream_check_provider" => match (app_arg(args, "appType"), str_arg(args, "providerId")) {
            (Ok(app_type), Ok(id)) => stream_check_provider(state, app_type, id),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // No args -> StreamCheckConfig object.
        "get_stream_check_config" => state
            .db
            .get_stream_check_config()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // Deserialize a structured arg -> void.
        "save_stream_check_config" => match from_arg::<StreamCheckConfig>(args, "config") {
            Ok(config) => state
                .db
                .save_stream_check_config(&config)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // stream_check_all_providers has no cc-switch-cli backing fn; it falls
        // through to 501.
        _ => return None,
    })
}

fn stream_check_provider(
    state: &AppState,
    app_type: crate::AppType,
    id: &str,
) -> Result<Value, WebError> {
    let providers = ProviderService::list(state, app_type.clone()).map_err(WebError::Domain)?;
    let provider = providers
        .get(id)
        .ok_or_else(|| WebError::Domain(AppError::Message(format!("Provider '{}' not found", id))))?
        .clone();
    let config: StreamCheckConfig = state
        .db
        .get_stream_check_config()
        .map_err(WebError::Domain)?;

    let result = super::common::block_on(async {
        StreamCheckService::check_with_retry(&app_type, &provider, &config).await
    })
    .map_err(WebError::Domain)?;

    let _ = state
        .db
        .save_stream_check_log(id, &provider.name, app_type.as_str(), &result);

    to_value(result)
}
