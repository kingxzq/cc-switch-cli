//! Subscription / coding-plan quota commands (`src/lib/api/subscription.ts`).
//!
//! Follows the [`super::providers`] template. These map to the async quota /
//! balance services in `crate::services::{subscription, coding_plan, balance}`,
//! which return `Result<T, String>`, so String errors are wrapped as
//! `AppError::Message`. The async fns are awaited via `common::block_on`.

use serde_json::Value;

use super::common::{block_on, opt_str_arg, str_arg, to_value};
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ tool }` -> SubscriptionQuota object.
        "get_subscription_quota" => match str_arg(args, "tool") {
            Ok(tool) => block_on(crate::services::subscription::get_subscription_quota(tool))
                .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ baseUrl, apiKey, accessKeyId?, secretAccessKey? }` -> SubscriptionQuota.
        // The cc-switch-cli service signature is (base_url, api_key); the optional
        // AK/SK signing args have no backing parameter, so they are ignored.
        "get_coding_plan_quota" => match (str_arg(args, "baseUrl"), str_arg(args, "apiKey")) {
            (Ok(base_url), Ok(api_key)) => {
                let _ = opt_str_arg(args, "accessKeyId");
                let _ = opt_str_arg(args, "secretAccessKey");
                block_on(crate::services::coding_plan::get_coding_plan_quota(
                    base_url, api_key,
                ))
                .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                .and_then(to_value)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ baseUrl, apiKey }` -> UsageResult object.
        "get_balance" => match (str_arg(args, "baseUrl"), str_arg(args, "apiKey")) {
            (Ok(base_url), Ok(api_key)) => {
                block_on(crate::services::balance::get_balance(base_url, api_key))
                    .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                    .and_then(to_value)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // get_codex_oauth_quota has no backing fn in cc-switch-cli -> fall through
        // to 501.
        _ => return None,
    })
}
