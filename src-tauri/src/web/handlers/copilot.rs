//! GitHub Copilot OAuth / quota commands (`src/lib/api/copilot.ts`).
//!
//! All flows live in [`CopilotAuthService`] (and its underlying
//! `CopilotAuthManager`). Every backing fn is async and returns
//! `Result<_, CopilotAuthError>` (a `thiserror` enum), so each arm wraps the
//! call in [`super::common::block_on`] and maps the error through
//! [`domain`] into `WebError::Domain(AppError::Message(..))`.
//!
//! `CopilotAuthService` exposes the token/model/usage/status flows as static
//! methods; the account-management flows (list/remove/set-default/logout/
//! is-authenticated) live on the manager, reached via
//! `CopilotAuthService::manager()`.

use serde_json::Value;

use super::common::{block_on, ok_null, str_arg, to_value};
use crate::services::CopilotAuthService;
use crate::web::error::WebError;
use crate::{AppError, AppState};

/// Map a `CopilotAuthError` into a domain `WebError`.
fn domain<E: std::fmt::Display>(err: E) -> WebError {
    WebError::Domain(AppError::Message(err.to_string()))
}

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> GitHubDeviceCodeResponse. Default github.com domain.
        "copilot_start_device_flow" => block_on(CopilotAuthService::start_device_flow(None))
            .map_err(domain)
            .and_then(to_value),

        // `{ deviceCode }` -> bool. Some(account) => success, None => still pending.
        "copilot_poll_for_auth" => match str_arg(args, "deviceCode") {
            Ok(device_code) => block_on(CopilotAuthService::poll_for_token(device_code))
                .map_err(domain)
                .map(|account| Value::Bool(account.is_some())),
            Err(e) => Err(e),
        },

        // No args -> CopilotAuthStatus.
        "copilot_get_auth_status" => to_value(block_on(CopilotAuthService::get_status())),

        // No args -> void. Clears all authenticated accounts.
        "copilot_logout" => block_on(CopilotAuthService::manager().clear_auth())
            .map_err(domain)
            .and_then(|_| ok_null()),

        // No args -> bool.
        "copilot_is_authenticated" => Ok(Value::Bool(block_on(
            CopilotAuthService::manager().is_authenticated(),
        ))),

        // No args -> string (default account's valid Copilot token).
        "copilot_get_token" => block_on(CopilotAuthService::get_valid_token())
            .map(Value::String)
            .map_err(domain),

        // No args -> CopilotModel[].
        "copilot_get_models" => block_on(CopilotAuthService::fetch_models())
            .map_err(domain)
            .and_then(to_value),

        // No args -> CopilotUsageResponse.
        "copilot_get_usage" => block_on(CopilotAuthService::fetch_usage())
            .map_err(domain)
            .and_then(to_value),

        // ==================== Multi-account ====================

        // No args -> GitHubAccount[].
        "copilot_list_accounts" => {
            to_value(block_on(CopilotAuthService::manager().list_accounts()))
        }

        // `{ deviceCode }` -> GitHubAccount | null.
        "copilot_poll_for_account" => match str_arg(args, "deviceCode") {
            Ok(device_code) => block_on(CopilotAuthService::poll_for_token(device_code))
                .map_err(domain)
                .and_then(|account| match account {
                    Some(account) => to_value(account),
                    None => Ok(Value::Null),
                }),
            Err(e) => Err(e),
        },

        // `{ accountId }` -> void.
        "copilot_remove_account" => match str_arg(args, "accountId") {
            Ok(account_id) => block_on(CopilotAuthService::manager().remove_account(account_id))
                .map_err(domain)
                .and_then(|_| ok_null()),
            Err(e) => Err(e),
        },

        // `{ accountId }` -> void.
        "copilot_set_default_account" => match str_arg(args, "accountId") {
            Ok(account_id) => {
                block_on(CopilotAuthService::manager().set_default_account(account_id))
                    .map_err(domain)
                    .and_then(|_| ok_null())
            }
            Err(e) => Err(e),
        },

        // `{ accountId }` -> string.
        "copilot_get_token_for_account" => match str_arg(args, "accountId") {
            Ok(account_id) => block_on(CopilotAuthService::get_valid_token_for_account(account_id))
                .map(Value::String)
                .map_err(domain),
            Err(e) => Err(e),
        },

        // `{ accountId }` -> CopilotModel[].
        "copilot_get_models_for_account" => match str_arg(args, "accountId") {
            Ok(account_id) => block_on(CopilotAuthService::fetch_models_for_account(account_id))
                .map_err(domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ accountId }` -> CopilotUsageResponse.
        "copilot_get_usage_for_account" => match str_arg(args, "accountId") {
            Ok(account_id) => block_on(CopilotAuthService::fetch_usage_for_account(account_id))
                .map_err(domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        _ => return None,
    })
}
