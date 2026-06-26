use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};

use super::error::WebError;
use super::handlers;
use super::state::WebState;

/// `POST /api/invoke/:command` — the bridge for the frontend's Tauri `invoke()`.
///
/// `command` is the snake_case command name; the JSON body is the args object
/// (camelCase keys) the TS code passes as the second `invoke()` argument. The
/// returned JSON value is what the TS promise resolves to. Dispatch itself lives
/// in [`handlers`]; this boundary adds the post-mutation SSE notifications.
pub async fn invoke_handler(
    State(state): State<WebState>,
    Path(command): Path<String>,
    Json(args): Json<Value>,
) -> Result<Json<Value>, WebError> {
    let result = handlers::dispatch(&state.app, &command, &args)?;
    emit_side_effects(&state, &command, &args);
    Ok(Json(result))
}

/// After a successful mutating command, push a Tauri-style event so the SPA
/// refreshes (the frontend listens for `provider-switched`).
fn emit_side_effects(state: &WebState, command: &str, args: &Value) {
    if command == "switch_provider" {
        let app = args.get("app").and_then(Value::as_str).unwrap_or_default();
        let id = args.get("id").and_then(Value::as_str).unwrap_or_default();
        let _ = state.events.send(
            json!({
                "event": "provider-switched",
                "payload": { "appType": app, "providerId": id }
            })
            .to_string(),
        );
    }
}
