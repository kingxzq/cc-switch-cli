//! Claude-specific and legacy/compat MCP config commands (`src/lib/api/mcp.ts`).
//!
//! These are the lower-level / Claude-specific MCP ops plus the deprecated
//! per-app config compat layer, distinct from the unified `get_mcp_servers`
//! family wired elsewhere. They mirror the desktop `commands/mcp.rs` handlers:
//!   - `get_claude_mcp_status` / `read_claude_mcp_config` /
//!     `upsert_claude_mcp_server` / `delete_claude_mcp_server` operate directly
//!     on `~/.claude.json` via [`crate::claude_mcp`].
//!   - `get_mcp_config` / `upsert_mcp_server_in_config` /
//!     `delete_mcp_server_in_config` / `set_mcp_enabled` are the deprecated
//!     per-app compat commands that translate to the unified [`McpService`].
//!
//! All backing fns are synchronous and return `Result<_, AppError>`.

use serde_json::Value;

use super::common::{app_arg, bool_arg, str_arg, to_value};
use crate::web::error::WebError;
use crate::{config, AppState, McpApps, McpServer, McpService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> McpStatus (camelCase). Reads ~/.claude.json directly.
        "get_claude_mcp_status" => crate::claude_mcp::get_mcp_status()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // No args -> string | null (raw mcp.json text, or null when absent).
        "read_claude_mcp_config" => match crate::claude_mcp::read_mcp_json() {
            Ok(Some(text)) => Ok(Value::String(text)),
            Ok(None) => Ok(Value::Null),
            Err(e) => Err(WebError::Domain(e)),
        },

        // `{ id, spec }` -> bool. spec is an arbitrary JSON server definition.
        "upsert_claude_mcp_server" => match (str_arg(args, "id"), args.get("spec")) {
            (Ok(id), Some(spec)) => crate::claude_mcp::upsert_mcp_server(id, spec.clone())
                .map(Value::Bool)
                .map_err(WebError::Domain),
            (Err(e), _) => Err(e),
            (_, None) => Err(WebError::BadRequest("missing 'spec' argument".into())),
        },

        // `{ id }` -> bool.
        "delete_claude_mcp_server" => match str_arg(args, "id") {
            Ok(id) => crate::claude_mcp::delete_mcp_server(id)
                .map(Value::Bool)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // ===== Deprecated per-app compat layer =====

        // `{ app }` -> McpConfigResponse { config_path, servers }.
        "get_mcp_config" => match app_arg(args, "app") {
            Ok(app_ty) => {
                let config_path = config::get_app_config_path().to_string_lossy().into_owned();
                #[allow(deprecated)]
                McpService::get_servers(state, app_ty)
                    .map_err(WebError::Domain)
                    .and_then(|servers| {
                        to_value(McpConfigResponse {
                            config_path,
                            servers,
                        })
                    })
            }
            Err(e) => Err(e),
        },

        // `{ app, id, spec, syncOtherSide? }` -> bool. Translates the legacy
        // per-app call into the unified McpService::upsert_server, matching the
        // desktop compat handler.
        "upsert_mcp_server_in_config" => {
            match (app_arg(args, "app"), str_arg(args, "id"), args.get("spec")) {
                (Ok(app_ty), Ok(id), Some(spec)) => {
                    let sync_other_side = bool_arg(args, "syncOtherSide", false);

                    // Reuse an existing unified server entry if present, else
                    // build a fresh one (name taken from spec.name, else id).
                    let existing = match McpService::get_all_servers(state) {
                        Ok(mut servers) => servers.remove(id),
                        Err(e) => return Some(Err(WebError::Domain(e))),
                    };

                    let mut new_server = if let Some(mut existing) = existing {
                        existing.server = spec.clone();
                        existing.apps.set_enabled_for(&app_ty, true);
                        existing
                    } else {
                        let mut apps = McpApps::default();
                        apps.set_enabled_for(&app_ty, true);
                        let name = spec
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(id)
                            .to_string();
                        McpServer {
                            id: id.to_string(),
                            name,
                            server: spec.clone(),
                            apps,
                            description: None,
                            homepage: None,
                            docs: None,
                            tags: Vec::new(),
                        }
                    };

                    if sync_other_side {
                        new_server.apps.claude = true;
                        new_server.apps.codex = true;
                        new_server.apps.gemini = true;
                        new_server.apps.opencode = true;
                    }

                    McpService::upsert_server(state, new_server)
                        .map(|_| Value::Bool(true))
                        .map_err(WebError::Domain)
                }
                (Err(e), _, _) | (_, Err(e), _) => Err(e),
                (_, _, None) => Err(WebError::BadRequest("missing 'spec' argument".into())),
            }
        }

        // `{ app, id, syncOtherSide? }` -> bool. The unified delete ignores the
        // app (deletes from all), matching the desktop compat handler.
        "delete_mcp_server_in_config" => match str_arg(args, "id") {
            Ok(id) => McpService::delete_server(state, id)
                .map(Value::Bool)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ app, id, enabled }` -> bool.
        "set_mcp_enabled" => match (app_arg(args, "app"), str_arg(args, "id")) {
            (Ok(app_ty), Ok(id)) => {
                let enabled = bool_arg(args, "enabled", false);
                #[allow(deprecated)]
                McpService::set_enabled(state, app_ty, id, enabled)
                    .map(Value::Bool)
                    .map_err(WebError::Domain)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        _ => return None,
    })
}

/// Mirrors the desktop `McpConfigResponse` so the deprecated `get_mcp_config`
/// returns `{ config_path, servers }` to the frontend.
#[derive(serde::Serialize)]
struct McpConfigResponse {
    config_path: String,
    servers: std::collections::HashMap<String, Value>,
}
