//! MCP server commands (`src/lib/api/mcp.ts`).
//!
//! Wires the v3.7.0 unified MCP management commands to [`McpService`]. The
//! deprecated v3.6.x per-app commands and the Claude-specific status/read
//! commands have no unified backing fn and fall through to 501.

use serde_json::{json, Value};

use super::common::{app_arg, bool_arg, from_arg, ok_null, str_arg, to_value};
use crate::web::error::WebError;
use crate::{AppState, McpServer, McpService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> HashMap<String, McpServer> serialized as McpServersMap.
        "get_mcp_servers" => McpService::get_all_servers(state)
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ server: McpServer }` -> void.
        "upsert_mcp_server" => match from_arg::<McpServer>(args, "server") {
            Ok(server) => McpService::upsert_server(state, server)
                .map_err(WebError::Domain)
                .and_then(|_| ok_null()),
            Err(e) => Err(e),
        },

        // `{ id }` -> bool (true when a server was removed).
        "delete_mcp_server" => match str_arg(args, "id") {
            Ok(id) => McpService::delete_server(state, id)
                .map(Value::Bool)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ serverId, app, enabled }` -> void.
        "toggle_mcp_app" => match (str_arg(args, "serverId"), app_arg(args, "app")) {
            (Ok(server_id), Ok(app_type)) => {
                let enabled = bool_arg(args, "enabled", false);
                McpService::toggle_app(state, server_id, app_type, enabled)
                    .map(|_| Value::Null)
                    .map_err(WebError::Domain)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // No args -> number (count of imported servers).
        "import_mcp_from_apps" => McpService::import_from_supported_apps(state)
            .map(|n| json!(n))
            .map_err(WebError::Domain),

        // `{ cmd }` -> bool. Pure PATH lookup, no domain state involved.
        "validate_mcp_command" => match str_arg(args, "cmd") {
            Ok(cmd) => Ok(Value::Bool(which::which(cmd).is_ok())),
            Err(e) => Err(e),
        },

        // Deprecated v3.6.x per-app commands (get_mcp_config,
        // upsert_mcp_server_in_config, delete_mcp_server_in_config,
        // set_mcp_enabled) and Claude-specific status/read commands
        // (get_claude_mcp_status, read_claude_mcp_config,
        // upsert_claude_mcp_server, delete_claude_mcp_server) have no unified
        // backing fn and fall through to 501.
        _ => return None,
    })
}

// Silence unused-import warning if `ok_null` is not referenced elsewhere; it is
// kept for parity with the other handler modules' void-returning commands.
#[allow(unused_imports)]
use ok_null as _ok_null_marker;
