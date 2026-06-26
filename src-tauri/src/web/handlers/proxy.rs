//! Local proxy control/status/config commands (`src/lib/api/proxy.ts`).
//!
//! Mirrors the desktop `commands/proxy.rs` Tauri handlers. Most proxy fns are
//! async on `ProxyService` / the proxy DAO, so they are wrapped with
//! [`super::common::block_on`]. Service fns return `Result<T, String>` while DAO
//! fns return `Result<T, AppError>`; the two are mapped to [`WebError`]
//! accordingly.

use serde_json::Value;

use super::common::{block_on, from_arg, ok_null, str_arg, to_value};
use crate::proxy::types::{AppProxyConfig, GlobalProxyConfig};
use crate::web::error::WebError;
use crate::{AppState, ProxyConfig};

/// Map a service-layer `String` error into a domain [`WebError`].
fn domain_str(e: String) -> WebError {
    WebError::Domain(crate::AppError::Message(e))
}

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // ===== 代理服务器控制 =====

        // No args -> ProxyServerInfo. Service fn returns Result<_, String>.
        "start_proxy_server" => block_on(async { state.proxy_service.start().await })
            .map_err(domain_str)
            .and_then(to_value),

        // No args -> void. Service fn returns Result<(), String>.
        "stop_proxy_with_restore" => {
            match block_on(async { state.proxy_service.stop_with_restore().await }) {
                Ok(()) => ok_null(),
                Err(e) => Err(domain_str(e)),
            }
        }

        // No args -> ProxyStatus. Infallible (returns ProxyStatus directly).
        "get_proxy_status" => to_value(block_on(async { state.proxy_service.get_status().await })),

        // No args -> bool. Infallible (returns bool directly).
        "is_proxy_running" => Ok(Value::Bool(block_on(async {
            state.proxy_service.is_running().await
        }))),

        // No args -> bool. DAO fn returns Result<bool, AppError>.
        "is_live_takeover_active" => block_on(async { state.db.is_live_takeover_active().await })
            .map(Value::Bool)
            .map_err(WebError::Domain),

        // `{ appType, providerId }` -> void. Mirrors desktop: block official
        // providers during proxy takeover, then hot-switch the target.
        "switch_proxy_provider" => match (str_arg(args, "appType"), str_arg(args, "providerId")) {
            (Ok(app_type), Ok(provider_id)) => {
                let provider = state
                    .db
                    .get_provider_by_id(provider_id, app_type)
                    .map_err(WebError::Domain)
                    .and_then(|p| {
                        p.ok_or_else(|| domain_str(format!("供应商不存在: {provider_id}")))
                    });
                match provider {
                    Ok(provider) => {
                        if provider.category.as_deref() == Some("official") {
                            Err(domain_str(
                                    "代理接管模式下不能切换到官方供应商 (Cannot switch to official provider during proxy takeover)"
                                        .to_string(),
                                ))
                        } else {
                            match block_on(async {
                                state
                                    .proxy_service
                                    .switch_proxy_target(app_type, provider_id)
                                    .await
                            }) {
                                Ok(()) => ok_null(),
                                Err(e) => Err(domain_str(e)),
                            }
                        }
                    }
                    Err(e) => Err(e),
                }
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // ===== 接管状态 =====

        // No args -> ProxyTakeoverStatus. Service fn returns Result<_, String>.
        "get_proxy_takeover_status" => {
            block_on(async { state.proxy_service.get_takeover_status().await })
                .map_err(domain_str)
                .and_then(to_value)
        }

        // `{ appType, enabled }` -> void. Service fn returns Result<(), String>.
        "set_proxy_takeover_for_app" => match str_arg(args, "appType") {
            Ok(app_type) => {
                let enabled = super::common::bool_arg(args, "enabled", false);
                match block_on(async {
                    state
                        .proxy_service
                        .set_takeover_for_app(app_type, enabled)
                        .await
                }) {
                    Ok(()) => ok_null(),
                    Err(e) => Err(domain_str(e)),
                }
            }
            Err(e) => Err(e),
        },

        // ===== Legacy 代理配置 (v2 兼容) =====

        // No args -> ProxyConfig. Service fn returns Result<_, AppError>.
        "get_proxy_config" => block_on(async { state.proxy_service.get_config().await })
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ config }` -> void. Service fn returns Result<(), AppError>.
        "update_proxy_config" => match from_arg::<ProxyConfig>(args, "config") {
            Ok(config) => {
                match block_on(async { state.proxy_service.update_config(&config).await }) {
                    Ok(()) => ok_null(),
                    Err(e) => Err(WebError::Domain(e)),
                }
            }
            Err(e) => Err(e),
        },

        // ===== v3+ 全局/应用级配置 =====

        // No args -> GlobalProxyConfig. DAO fn returns Result<_, AppError>.
        "get_global_proxy_config" => block_on(async { state.db.get_global_proxy_config().await })
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ config }` -> void. DAO fn returns Result<(), AppError>.
        "update_global_proxy_config" => match from_arg::<GlobalProxyConfig>(args, "config") {
            Ok(config) => {
                match block_on(async { state.db.update_global_proxy_config(config).await }) {
                    Ok(()) => ok_null(),
                    Err(e) => Err(WebError::Domain(e)),
                }
            }
            Err(e) => Err(e),
        },

        // `{ appType }` -> AppProxyConfig. DAO fn returns Result<_, AppError>.
        "get_proxy_config_for_app" => match str_arg(args, "appType") {
            Ok(app_type) => block_on(async { state.db.get_proxy_config_for_app(app_type).await })
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ config }` -> void (config carries its own app_type). DAO fn returns
        // Result<(), AppError>.
        "update_proxy_config_for_app" => match from_arg::<AppProxyConfig>(args, "config") {
            Ok(config) => {
                match block_on(async { state.db.update_proxy_config_for_app(config).await }) {
                    Ok(()) => ok_null(),
                    Err(e) => Err(WebError::Domain(e)),
                }
            }
            Err(e) => Err(e),
        },

        // ===== 计费默认配置 =====

        // `{ appType }` -> string. DAO fn returns Result<String, AppError>.
        "get_default_cost_multiplier" => match str_arg(args, "appType") {
            Ok(app_type) => {
                block_on(async { state.db.get_default_cost_multiplier(app_type).await })
                    .map(Value::String)
                    .map_err(WebError::Domain)
            }
            Err(e) => Err(e),
        },

        // `{ appType, value }` -> void. DAO fn returns Result<(), AppError>.
        "set_default_cost_multiplier" => match (str_arg(args, "appType"), str_arg(args, "value")) {
            (Ok(app_type), Ok(value)) => match block_on(async {
                state.db.set_default_cost_multiplier(app_type, value).await
            }) {
                Ok(()) => ok_null(),
                Err(e) => Err(WebError::Domain(e)),
            },
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ appType }` -> string. DAO fn returns Result<String, AppError>.
        "get_pricing_model_source" => match str_arg(args, "appType") {
            Ok(app_type) => block_on(async { state.db.get_pricing_model_source(app_type).await })
                .map(Value::String)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ appType, value }` -> void. DAO fn returns Result<(), AppError>.
        "set_pricing_model_source" => match (str_arg(args, "appType"), str_arg(args, "value")) {
            (Ok(app_type), Ok(value)) => {
                match block_on(async { state.db.set_pricing_model_source(app_type, value).await }) {
                    Ok(()) => ok_null(),
                    Err(e) => Err(WebError::Domain(e)),
                }
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        _ => return None,
    })
}
