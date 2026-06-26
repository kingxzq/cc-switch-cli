//! Failover commands (`src/lib/api/failover.ts`).
//!
//! Covers the circuit-breaker surface (provider health, breaker config/stats)
//! and the failover provider queue (list/available/add/remove, auto-failover
//! toggle). Follows the [`super::meta`] template.
//!
//! Failover queue DAO fns (`get_failover_queue`, `add_to_failover_queue`, ...)
//! are synchronous; the proxy-config / circuit-breaker DAO fns and the proxy
//! service fns are async and run via [`super::common::block_on`].

use serde_json::Value;

use super::common::{app_arg, block_on, from_arg, ok_null, str_arg, to_value};
use crate::proxy::circuit_breaker::CircuitBreakerConfig;
use crate::web::error::WebError;
use crate::{AppError, AppState};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // ---------- Circuit breaker ----------

        // { providerId, appType } -> ProviderHealth object.
        "get_provider_health" => match (str_arg(args, "providerId"), app_arg(args, "appType")) {
            (Ok(provider_id), Ok(app_type)) => block_on(async {
                state
                    .db
                    .get_provider_health(provider_id, app_type.as_str())
                    .await
            })
            .map_err(WebError::Domain)
            .and_then(to_value),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // { providerId, appType } -> void. Resets the DB health record and the
        // in-memory breaker. The desktop command additionally performs an
        // automatic recovery switch via a Tauri-only FailoverSwitchManager; that
        // step is desktop-specific and omitted here.
        "reset_circuit_breaker" => match (str_arg(args, "providerId"), app_arg(args, "appType")) {
            (Ok(provider_id), Ok(app_type)) => block_on(async {
                state
                    .db
                    .update_provider_health(provider_id, app_type.as_str(), true, None)
                    .await
                    .map_err(WebError::Domain)?;
                state
                    .proxy_service
                    .reset_provider_circuit_breaker(provider_id, app_type.as_str())
                    .await
                    .map_err(|e| WebError::Domain(AppError::Message(e)))?;
                ok_null()
            }),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // No args -> CircuitBreakerConfig object.
        "get_circuit_breaker_config" => {
            block_on(async { state.db.get_circuit_breaker_config().await })
                .map_err(WebError::Domain)
                .and_then(to_value)
        }

        // { config } -> void. Persists then hot-updates the running proxy.
        "update_circuit_breaker_config" => match from_arg::<CircuitBreakerConfig>(args, "config") {
            Ok(config) => block_on(async {
                state
                    .db
                    .update_circuit_breaker_config(&config)
                    .await
                    .map_err(WebError::Domain)?;
                state
                    .proxy_service
                    .update_circuit_breaker_configs(config)
                    .await
                    .map_err(|e| WebError::Domain(AppError::Message(e)))?;
                ok_null()
            }),
            Err(e) => Err(e),
        },

        // { providerId, appType } -> CircuitBreakerStats | null. The desktop
        // command currently always returns None (live breaker stats are not
        // exposed); mirror that.
        "get_circuit_breaker_stats" => {
            match (str_arg(args, "providerId"), app_arg(args, "appType")) {
                (Ok(_), Ok(_)) => Ok(Value::Null),
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // ---------- Failover queue ----------

        // { appType } -> FailoverQueueItem[].
        "get_failover_queue" => match app_arg(args, "appType") {
            Ok(app_type) => state
                .db
                .get_failover_queue(app_type.as_str())
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // { appType } -> Provider[].
        "get_available_providers_for_failover" => match app_arg(args, "appType") {
            Ok(app_type) => state
                .db
                .get_available_providers_for_failover(app_type.as_str())
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // { appType, providerId } -> void.
        "add_to_failover_queue" => match (app_arg(args, "appType"), str_arg(args, "providerId")) {
            (Ok(app_type), Ok(provider_id)) => state
                .db
                .add_to_failover_queue(app_type.as_str(), provider_id)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // { appType, providerId } -> void.
        "remove_from_failover_queue" => {
            match (app_arg(args, "appType"), str_arg(args, "providerId")) {
                (Ok(app_type), Ok(provider_id)) => state
                    .db
                    .remove_from_failover_queue(app_type.as_str(), provider_id)
                    .map(|_| Value::Null)
                    .map_err(WebError::Domain),
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // { appType } -> bool (reads proxy_config.auto_failover_enabled).
        "get_auto_failover_enabled" => match app_arg(args, "appType") {
            Ok(app_type) => {
                block_on(async { state.db.get_proxy_config_for_app(app_type.as_str()).await })
                    .map(|config| Value::Bool(config.auto_failover_enabled))
                    .map_err(WebError::Domain)
            }
            Err(e) => Err(e),
        },

        // { appType, enabled } -> void. Delegates to the proxy service, which
        // enforces the same enable-path guards (proxy takeover active, queue
        // ready) and persists the toggle. The desktop command additionally
        // emits a `provider-switched` event and refreshes the tray; those are
        // desktop-only side effects and omitted here.
        "set_auto_failover_enabled" => {
            match (
                app_arg(args, "appType"),
                args.get("enabled").and_then(Value::as_bool),
            ) {
                (Ok(app_type), Some(enabled)) => block_on(async {
                    state
                        .proxy_service
                        .set_auto_failover_for_app(app_type.as_str(), enabled)
                        .await
                })
                .map(|_| Value::Null)
                .map_err(|e| WebError::Domain(AppError::Message(e))),
                (Err(e), _) => Err(e),
                (_, None) => Err(WebError::BadRequest(
                    "missing 'enabled' argument".to_string(),
                )),
            }
        }

        _ => return None,
    })
}
