//! Hermes commands (`src/lib/api/hermes.ts`).
//!
//! Follows the [`super::meta`] / [`super::providers`] templates. CC Switch keeps
//! its Hermes surface minimal: the web UI / dashboard launcher are desktop-only
//! and fall through to 501; only the local-file memory blobs and the read-only
//! `model:` snapshot are mapped here.

use serde_json::Value;

use super::common::{bool_arg, from_arg, str_arg, to_value};
use crate::hermes_config::{
    self, read_memory, read_memory_limits, set_memory_enabled, write_memory, MemoryKind,
};
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> HermesModelConfig | null.
        "get_hermes_model_config" => hermes_config::get_model_config()
            .map_err(WebError::Domain)
            .and_then(|opt| match opt {
                Some(model) => to_value(model),
                None => Ok(Value::Null),
            }),

        // `{ kind }` arg -> string (empty when the file is absent).
        "get_hermes_memory" => match from_arg::<MemoryKind>(args, "kind") {
            Ok(kind) => read_memory(kind)
                .map(Value::String)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ kind, content }` args -> void.
        "set_hermes_memory" => match (
            from_arg::<MemoryKind>(args, "kind"),
            str_arg(args, "content"),
        ) {
            (Ok(kind), Ok(content)) => write_memory(kind, content)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // No args -> HermesMemoryLimits.
        "get_hermes_memory_limits" => read_memory_limits()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ kind, enabled }` args -> void.
        "set_hermes_memory_enabled" => match from_arg::<MemoryKind>(args, "kind") {
            Ok(kind) => set_memory_enabled(kind, bool_arg(args, "enabled", false))
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // Desktop-only (system browser / terminal launch): open_hermes_web_ui,
        // launch_hermes_dashboard. They fall through to 501.
        _ => return None,
    })
}
