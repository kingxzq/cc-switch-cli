//! VS Code integration commands (`src/lib/api/vscode.ts`).
//!
//! Despite the "vscode" name, the mappable commands here are live-config reads,
//! endpoint latency tests, and custom-endpoint CRUD — all backed by real
//! cc-switch-cli domain fns. The file-dialog commands are desktop-only (native
//! OS dialogs) and `import_config_from_file` has no CLI backing, so those fall
//! through to HTTP 501.
//!
//! Follows the [`super::providers`] template.

use serde_json::Value;

use super::common::{app, block_on, from_arg, str_arg, to_value};
use crate::web::error::WebError;
use crate::{AppState, ProviderService, SpeedtestService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ app }` -> live config JSON object.
        "read_live_provider_settings" => match app(args) {
            Ok(t) => ProviderService::read_live_settings(t).map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ urls: string[], timeoutSecs?: number }` -> EndpointLatencyResult[].
        "test_api_endpoints" => match from_arg::<Vec<String>>(args, "urls") {
            Ok(urls) => {
                let timeout_secs = match super::common::opt_from_arg::<u64>(args, "timeoutSecs") {
                    Ok(v) => v,
                    Err(e) => return Some(Err(e)),
                };
                block_on(async { SpeedtestService::test_endpoints(urls, timeout_secs).await })
                    .map_err(WebError::Domain)
                    .and_then(to_value)
            }
            Err(e) => Err(e),
        },

        // `{ app, providerId }` -> CustomEndpoint[].
        "get_custom_endpoints" => match (app(args), str_arg(args, "providerId")) {
            (Ok(t), Ok(provider_id)) => {
                ProviderService::get_custom_endpoints(state, t, provider_id)
                    .map_err(WebError::Domain)
                    .and_then(to_value)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ app, providerId, url }` -> void.
        "add_custom_endpoint" => {
            match (app(args), str_arg(args, "providerId"), str_arg(args, "url")) {
                (Ok(t), Ok(provider_id), Ok(url)) => {
                    ProviderService::add_custom_endpoint(state, t, provider_id, url.to_string())
                        .map(|_| Value::Null)
                        .map_err(WebError::Domain)
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
            }
        }

        // `{ app, providerId, url }` -> void.
        "remove_custom_endpoint" => {
            match (app(args), str_arg(args, "providerId"), str_arg(args, "url")) {
                (Ok(t), Ok(provider_id), Ok(url)) => {
                    ProviderService::remove_custom_endpoint(state, t, provider_id, url.to_string())
                        .map(|_| Value::Null)
                        .map_err(WebError::Domain)
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
            }
        }

        // `{ app, providerId, url }` -> void.
        "update_endpoint_last_used" => {
            match (app(args), str_arg(args, "providerId"), str_arg(args, "url")) {
                (Ok(t), Ok(provider_id), Ok(url)) => ProviderService::update_endpoint_last_used(
                    state,
                    t,
                    provider_id,
                    url.to_string(),
                )
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
            }
        }

        // export_config_to_file is owned by the `settings` module (earlier in
        // the dispatch chain). import_config_from_file has no CLI wrapper, and
        // save_file_dialog / open_file_dialog are desktop-only native dialogs —
        // all fall through to HTTP 501.
        _ => return None,
    })
}
