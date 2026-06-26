//! Deep-link import commands (`src/lib/api/deeplink.ts`).
//!
//! Implements the `ccswitch://` parse + unified-import flow. Follows the
//! [`super::providers`] template: each command reads its args, calls the
//! cc-switch-cli `deeplink` domain fns, and shapes the JSON the frontend
//! expects.
//!
//! `merge_deeplink_config` is intentionally not wired — its only backing fn
//! (`deeplink::provider::parse_and_merge_config`) is private to the deeplink
//! module and not re-exported, so it cannot be called from here.

use serde_json::{json, Value};

use super::common::{from_arg, str_arg, to_value};
use crate::web::error::WebError;
use crate::{
    import_mcp_from_deeplink, import_prompt_from_deeplink, import_provider_from_deeplink,
    import_skill_from_deeplink, parse_deeplink_url, AppState, DeepLinkImportRequest,
};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ url: string }` -> DeepLinkImportRequest object (camelCase).
        "parse_deeplink" => match str_arg(args, "url") {
            Ok(url) => parse_deeplink_url(url)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ request: DeepLinkImportRequest }` -> tagged ImportResult.
        // Mirrors `cli/commands/deeplink.rs`: dispatch on `request.resource`.
        "import_from_deeplink_unified" => {
            match from_arg::<DeepLinkImportRequest>(args, "request") {
                Ok(request) => import_unified(state, request),
                Err(e) => Err(e),
            }
        }

        _ => return None,
    })
}

/// Dispatch a parsed [`DeepLinkImportRequest`] by resource type and build the
/// tagged `ImportResult` the frontend expects.
fn import_unified(state: &AppState, request: DeepLinkImportRequest) -> Result<Value, WebError> {
    match request.resource.as_str() {
        "provider" => import_provider_from_deeplink(state, request)
            .map(|id| json!({ "type": "provider", "id": id }))
            .map_err(WebError::Domain),
        "prompt" => import_prompt_from_deeplink(state, request)
            .map(|id| json!({ "type": "prompt", "id": id }))
            .map_err(WebError::Domain),
        "mcp" => import_mcp_from_deeplink(state, request)
            .map_err(WebError::Domain)
            .and_then(|result| {
                // McpImportResult serializes camelCase (importedCount,
                // importedIds, failed); merge in the `type` discriminant.
                let mut value = to_value(result)?;
                if let Value::Object(map) = &mut value {
                    map.insert("type".to_string(), Value::String("mcp".to_string()));
                }
                Ok(value)
            }),
        "skill" => import_skill_from_deeplink(state, request)
            .map(|key| json!({ "type": "skill", "key": key }))
            .map_err(WebError::Domain),
        other => Err(WebError::BadRequest(format!(
            "Unsupported resource type: {other}"
        ))),
    }
}
