//! Usage events / stats commands (`src/lib/api/usage.ts`).
//!
//! Follows the [`super::meta`] template. Covers the provider usage-script
//! queries (async, via [`ProviderService`]) and the proxy usage statistics
//! (sync, via [`crate::database::Database`] methods on `state.db` and the
//! session-usage services).
//!
//! Commands without a clean backing fn in cc-switch-cli (e.g. listing the model
//! pricing catalog, which the CLI only does through TUI-private raw SQL) fall
//! through to 501 — see the trailing comment.

use serde_json::Value;

use super::common::{app, opt_str_arg, str_arg, to_value};
use crate::database::ModelPricingUpdate;
use crate::services::usage_stats::LogFilters;
use crate::web::error::WebError;
use crate::{AppState, ProviderService};

/// Optional `i64` arg (timestamps / numeric filters). None when absent or null.
fn opt_i64_arg(args: &Value, key: &str) -> Option<i64> {
    args.get(key).and_then(Value::as_i64)
}

/// Optional `u64` arg, defaulting to `default` when absent or not a number.
fn u64_arg(args: &Value, key: &str, default: u64) -> u64 {
    args.get(key).and_then(Value::as_u64).unwrap_or(default)
}

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // --- Provider usage script methods (async) ---

        // queryProviderUsage({ providerId, app }) -> UsageResult
        // ProviderService::query_provider_usage returns Result<UsageResult, String>.
        "queryProviderUsage" => match (app(args), str_arg(args, "providerId")) {
            (Ok(app_type), Ok(provider_id)) => super::common::block_on(async {
                ProviderService::query_provider_usage(state, app_type, provider_id).await
            })
            .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
            .and_then(to_value),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // testUsageScript({ providerId, app, scriptCode, timeout?, apiKey?,
        //   baseUrl?, accessToken?, userId?, templateType? }) -> UsageResult
        "testUsageScript" => match (
            app(args),
            str_arg(args, "providerId"),
            str_arg(args, "scriptCode"),
        ) {
            (Ok(app_type), Ok(provider_id), Ok(script_code)) => {
                let timeout = u64_arg(args, "timeout", 10);
                let api_key = opt_str_arg(args, "apiKey");
                let base_url = opt_str_arg(args, "baseUrl");
                let access_token = opt_str_arg(args, "accessToken");
                let user_id = opt_str_arg(args, "userId");
                let template_type = opt_str_arg(args, "templateType");
                super::common::block_on(async {
                    ProviderService::test_usage_script(
                        state,
                        app_type,
                        provider_id,
                        script_code,
                        timeout,
                        api_key,
                        base_url,
                        access_token,
                        user_id,
                        template_type,
                    )
                    .await
                })
                .map_err(WebError::Domain)
                .and_then(to_value)
            }
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
        },

        // --- Proxy usage statistics methods (sync, on state.db) ---
        // The cc-switch-cli Database methods only filter by (start, end, app_type);
        // the frontend's extra providerName/model args are ignored.

        // get_usage_summary({ startDate?, endDate?, appType?, ... }) -> UsageSummary
        "get_usage_summary" => state
            .db
            .get_usage_summary(
                opt_i64_arg(args, "startDate"),
                opt_i64_arg(args, "endDate"),
                opt_str_arg(args, "appType"),
            )
            .map_err(WebError::Domain)
            .and_then(to_value),

        // get_usage_summary_by_app({ startDate?, endDate?, ... }) -> UsageSummaryByApp[]
        "get_usage_summary_by_app" => state
            .db
            .get_usage_summary_by_app(opt_i64_arg(args, "startDate"), opt_i64_arg(args, "endDate"))
            .map_err(WebError::Domain)
            .and_then(to_value),

        // get_usage_trends({ startDate?, endDate?, appType?, ... }) -> DailyStats[]
        "get_usage_trends" => state
            .db
            .get_daily_trends(
                opt_i64_arg(args, "startDate"),
                opt_i64_arg(args, "endDate"),
                opt_str_arg(args, "appType"),
            )
            .map_err(WebError::Domain)
            .and_then(to_value),

        // get_provider_stats({ startDate?, endDate?, appType?, ... }) -> ProviderStats[]
        "get_provider_stats" => state
            .db
            .get_provider_stats(
                opt_i64_arg(args, "startDate"),
                opt_i64_arg(args, "endDate"),
                opt_str_arg(args, "appType"),
            )
            .map_err(WebError::Domain)
            .and_then(to_value),

        // get_model_stats({ startDate?, endDate?, appType?, ... }) -> ModelStats[]
        "get_model_stats" => state
            .db
            .get_model_stats(
                opt_i64_arg(args, "startDate"),
                opt_i64_arg(args, "endDate"),
                opt_str_arg(args, "appType"),
            )
            .map_err(WebError::Domain)
            .and_then(to_value),

        // get_request_logs({ filters, page, pageSize }) -> PaginatedLogs
        "get_request_logs" => match super::common::from_arg::<LogFilters>(args, "filters") {
            Ok(filters) => {
                let page = u64_arg(args, "page", 0) as u32;
                let page_size = u64_arg(args, "pageSize", 20) as u32;
                state
                    .db
                    .get_request_logs(&filters, page, page_size)
                    .map_err(WebError::Domain)
                    .and_then(to_value)
            }
            Err(e) => Err(e),
        },

        // get_request_detail({ requestId }) -> RequestLog | null
        "get_request_detail" => match str_arg(args, "requestId") {
            Ok(request_id) => state
                .db
                .get_request_detail(request_id)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // update_model_pricing({ modelId, displayName, inputCost, outputCost,
        //   cacheReadCost, cacheCreationCost }) -> void
        "update_model_pricing" => match (
            str_arg(args, "modelId"),
            str_arg(args, "displayName"),
            str_arg(args, "inputCost"),
            str_arg(args, "outputCost"),
            str_arg(args, "cacheReadCost"),
            str_arg(args, "cacheCreationCost"),
        ) {
            (
                Ok(model_id),
                Ok(display_name),
                Ok(input_cost),
                Ok(output_cost),
                Ok(cache_read_cost),
                Ok(cache_creation_cost),
            ) => ModelPricingUpdate::new(
                model_id,
                display_name,
                input_cost,
                output_cost,
                cache_read_cost,
                cache_creation_cost,
            )
            .and_then(|pricing| {
                state.db.upsert_model_pricing(&pricing)?;
                // Best-effort backfill of historical zero-cost rows for this model.
                let _ = state
                    .db
                    .backfill_missing_usage_costs_for_model(&pricing.model_id);
                Ok(())
            })
            .map(|_| Value::Null)
            .map_err(WebError::Domain),
            (Err(e), ..)
            | (_, Err(e), ..)
            | (_, _, Err(e), ..)
            | (_, _, _, Err(e), ..)
            | (_, _, _, _, Err(e), _)
            | (_, _, _, _, _, Err(e)) => Err(e),
        },

        // delete_model_pricing({ modelId }) -> void
        "delete_model_pricing" => match str_arg(args, "modelId") {
            Ok(model_id) => state
                .db
                .delete_model_pricing(model_id)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // check_provider_limits({ providerId, appType }) -> ProviderLimitStatus
        "check_provider_limits" => match (str_arg(args, "providerId"), str_arg(args, "appType")) {
            (Ok(provider_id), Ok(app_type)) => state
                .db
                .check_provider_limits(provider_id, app_type)
                .map_err(WebError::Domain)
                .and_then(to_value),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // --- Session usage sync ---

        // sync_session_usage() -> SessionSyncResult
        // Mirrors the desktop command: sync Claude/Codex/Gemini/OpenCode logs and
        // merge their results, recording per-source failures as error strings.
        "sync_session_usage" => {
            match crate::services::session_usage::sync_claude_session_logs(&state.db) {
                Ok(mut result) => {
                    match crate::services::session_usage_codex::sync_codex_usage(&state.db) {
                        Ok(r) => result.merge(r),
                        Err(e) => result.errors.push(format!("Codex sync failed: {e}")),
                    }
                    match crate::services::session_usage_gemini::sync_gemini_usage(&state.db) {
                        Ok(r) => result.merge(r),
                        Err(e) => result.errors.push(format!("Gemini sync failed: {e}")),
                    }
                    match crate::services::session_usage_opencode::sync_opencode_usage(&state.db) {
                        Ok(r) => result.merge(r),
                        Err(e) => result.errors.push(format!("OpenCode sync failed: {e}")),
                    }
                    to_value(result)
                }
                Err(e) => Err(WebError::Domain(e)),
            }
        }

        // get_usage_data_sources() -> DataSourceSummary[]
        "get_usage_data_sources" => {
            crate::services::session_usage::get_data_source_breakdown(&state.db)
                .map_err(WebError::Domain)
                .and_then(to_value)
        }

        // get_model_pricing has no clean cc-switch-cli backing fn — the catalog is
        // only listed via TUI-private raw SQL — so it falls through to 501.
        _ => return None,
    })
}
