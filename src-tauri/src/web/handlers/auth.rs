//! Managed auth commands (`src/lib/api/auth.ts`).
//!
//! Maps the Codex OAuth account flows to [`AuthService`]. Every service fn is
//! async and returns `Result<_, String>`, so each arm wraps the call in
//! [`super::common::block_on`] and maps the string error into a domain error.
//!
//! Note: the frontend passes `githubDomain` for the `github_copilot` provider,
//! but cc-switch-cli's `AuthService` only supports `codex_oauth` and ignores
//! that arg — so the optional `githubDomain` is intentionally not forwarded.

use serde_json::Value;

use super::common::{block_on, ok_null, str_arg, to_value};
use crate::services::AuthService;
use crate::web::error::WebError;
use crate::{AppError, AppState};

/// Map an `AuthService` string error into a domain `WebError`.
fn domain(message: String) -> WebError {
    WebError::Domain(AppError::Message(message))
}

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ authProvider }` -> ManagedAuthDeviceCodeResponse. githubDomain is
        // ignored (codex_oauth only).
        "auth_start_login" => match str_arg(args, "authProvider") {
            Ok(provider) => block_on(AuthService::start_login(provider))
                .map_err(domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ authProvider, deviceCode }` -> ManagedAuthAccount | null.
        "auth_poll_for_account" => {
            match (str_arg(args, "authProvider"), str_arg(args, "deviceCode")) {
                (Ok(provider), Ok(device_code)) => {
                    block_on(AuthService::poll_for_account(provider, device_code))
                        .map_err(domain)
                        .and_then(|account| match account {
                            Some(account) => to_value(account),
                            None => Ok(Value::Null),
                        })
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // `{ authProvider }` -> ManagedAuthAccount[].
        "auth_list_accounts" => match str_arg(args, "authProvider") {
            Ok(provider) => block_on(AuthService::list_accounts(provider))
                .map_err(domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ authProvider }` -> ManagedAuthStatus.
        "auth_get_status" => match str_arg(args, "authProvider") {
            Ok(provider) => block_on(AuthService::get_status(provider))
                .map_err(domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ authProvider, accountId }` -> void.
        "auth_remove_account" => {
            match (str_arg(args, "authProvider"), str_arg(args, "accountId")) {
                (Ok(provider), Ok(account_id)) => {
                    block_on(AuthService::remove_account(provider, account_id))
                        .map_err(domain)
                        .and_then(|_| ok_null())
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // `{ authProvider, accountId }` -> void.
        "auth_set_default_account" => {
            match (str_arg(args, "authProvider"), str_arg(args, "accountId")) {
                (Ok(provider), Ok(account_id)) => {
                    block_on(AuthService::set_default_account(provider, account_id))
                        .map_err(domain)
                        .and_then(|_| ok_null())
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // `{ authProvider }` -> void.
        "auth_logout" => match str_arg(args, "authProvider") {
            Ok(provider) => block_on(AuthService::logout(provider))
                .map_err(domain)
                .and_then(|_| ok_null()),
            Err(e) => Err(e),
        },

        _ => return None,
    })
}
