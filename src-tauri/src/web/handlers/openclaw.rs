//! OpenClaw config commands (`src/lib/api/openclaw.ts`).
//!
//! Maps the `openclawApi` Tauri invokes to the synchronous
//! [`crate::openclaw_config`] helpers. These read/write sections of
//! `~/.openclaw/openclaw.json` (agents.defaults, env, tools, models.providers)
//! via process-global file access, so they ignore `state`.
//!
//! Follows the [`super::meta`] template: no-arg getters serialize their result
//! with `to_value`; setters deserialize a structured arg with `from_arg` and
//! return the `OpenClawWriteOutcome`.

use std::collections::HashMap;

use serde_json::Value;

use super::common::{from_arg, str_arg, to_value};
use crate::openclaw_config::{
    self, OpenClawAgentsDefaults, OpenClawDefaultModel, OpenClawEnvConfig,
    OpenClawModelCatalogEntry, OpenClawToolsConfig,
};
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> OpenClawDefaultModel | null.
        "get_openclaw_default_model" => openclaw_config::get_default_model()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ model }` -> OpenClawWriteOutcome.
        "set_openclaw_default_model" => match from_arg::<OpenClawDefaultModel>(args, "model") {
            Ok(model) => openclaw_config::set_default_model(&model)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // No args -> Record<string, OpenClawModelCatalogEntry> | null.
        "get_openclaw_model_catalog" => openclaw_config::get_model_catalog()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ catalog }` -> OpenClawWriteOutcome.
        "set_openclaw_model_catalog" => {
            match from_arg::<HashMap<String, OpenClawModelCatalogEntry>>(args, "catalog") {
                Ok(catalog) => openclaw_config::set_model_catalog(&catalog)
                    .map_err(WebError::Domain)
                    .and_then(to_value),
                Err(e) => Err(e),
            }
        }

        // No args -> OpenClawAgentsDefaults | null.
        "get_openclaw_agents_defaults" => openclaw_config::get_agents_defaults()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ defaults }` -> OpenClawWriteOutcome.
        "set_openclaw_agents_defaults" => {
            match from_arg::<OpenClawAgentsDefaults>(args, "defaults") {
                Ok(defaults) => openclaw_config::set_agents_defaults(&defaults)
                    .map_err(WebError::Domain)
                    .and_then(to_value),
                Err(e) => Err(e),
            }
        }

        // No args -> OpenClawEnvConfig.
        "get_openclaw_env" => openclaw_config::get_env_config()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ env }` -> OpenClawWriteOutcome.
        "set_openclaw_env" => match from_arg::<OpenClawEnvConfig>(args, "env") {
            Ok(env) => openclaw_config::set_env_config(&env)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // No args -> OpenClawToolsConfig.
        "get_openclaw_tools" => openclaw_config::get_tools_config()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ tools }` -> OpenClawWriteOutcome.
        "set_openclaw_tools" => match from_arg::<OpenClawToolsConfig>(args, "tools") {
            Ok(tools) => openclaw_config::set_tools_config(&tools)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // No args -> OpenClawHealthWarning[].
        "scan_openclaw_config_health" => openclaw_config::scan_openclaw_config_health()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ providerId }` -> Record<string, unknown> | null (live config read).
        "get_openclaw_live_provider" => match str_arg(args, "providerId") {
            Ok(id) => openclaw_config::get_provider(id)
                .map_err(WebError::Domain)
                .map(|v| v.unwrap_or(Value::Null)),
            Err(e) => Err(e),
        },

        _ => return None,
    })
}
