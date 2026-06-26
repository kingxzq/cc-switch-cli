//! Saved assistant session commands (`src/lib/api/sessions.ts`).
//!
//! Follows the [`super::meta`] / [`super::providers`] templates. Maps onto the
//! `crate::session_manager` module, whose fns return either plain values or
//! `Result<_, String>` (not `AppError`), so string errors are wrapped via
//! `AppError::Message`.

use serde_json::Value;

use super::common::{from_arg, str_arg, to_value};
use crate::session_manager::{self, DeleteSessionRequest};
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // No args -> SessionMeta[]. Scans every supported provider.
        "list_sessions" => to_value(session_manager::scan_sessions()),

        // `{ providerId, sourcePath }` -> SessionMessage[].
        "get_session_messages" => {
            match (str_arg(args, "providerId"), str_arg(args, "sourcePath")) {
                (Ok(provider_id), Ok(source_path)) => {
                    session_manager::load_messages(provider_id, source_path)
                        .map_err(string_err)
                        .and_then(to_value)
                }
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // `{ providerId, sessionId, sourcePath }` -> bool.
        "delete_session" => match (
            str_arg(args, "providerId"),
            str_arg(args, "sessionId"),
            str_arg(args, "sourcePath"),
        ) {
            (Ok(provider_id), Ok(session_id), Ok(source_path)) => {
                session_manager::delete_session(provider_id, session_id, source_path)
                    .map(Value::Bool)
                    .map_err(string_err)
            }
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
        },

        // `{ items: DeleteSessionOptions[] }` -> DeleteSessionResult[].
        "delete_sessions" => match from_arg::<Vec<DeleteSessionRequest>>(args, "items") {
            Ok(items) => to_value(session_manager::delete_sessions(&items)),
            Err(e) => Err(e),
        },

        // launch_session_terminal is desktop-only: it shells out to a macOS
        // terminal app via AppleScript and reads the preferred-terminal setting.
        // Falls through to 501.
        _ => return None,
    })
}

/// Wrap a `String` error from `session_manager` into a [`WebError`].
fn string_err(e: String) -> WebError {
    WebError::Domain(crate::AppError::Message(e))
}
