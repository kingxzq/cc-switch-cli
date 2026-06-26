//! OpenClaw workspace file + daily memory commands (`src/lib/api/workspace.ts`).
//!
//! Follows the [`super::meta`] / [`super::providers`] templates. Maps the
//! frontend `workspaceApi` invokes to the synchronous helpers in
//! [`crate::commands::workspace`], which restrict access to the OpenClaw
//! workspace allowlist and daily-memory files. Each helper returns
//! `Result<T, String>`, so failures are wrapped as `AppError::Message`.

use serde_json::Value;

use super::common::{str_arg, to_value};
use crate::web::error::WebError;
use crate::{commands, AppError, AppState};

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ filename }` -> string | null (Option<String> serializes to null).
        "read_workspace_file" => match str_arg(args, "filename") {
            Ok(filename) => commands::workspace::read_workspace_file(filename.to_string())
                .map_err(string_err)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ filename, content }` -> void.
        "write_workspace_file" => match (str_arg(args, "filename"), str_arg(args, "content")) {
            (Ok(filename), Ok(content)) => {
                commands::workspace::write_workspace_file(filename.to_string(), content.to_string())
                    .map(|_| Value::Null)
                    .map_err(string_err)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // No args -> array of DailyMemoryFileInfo (camelCase to match TS).
        "list_daily_memory_files" => commands::workspace::list_daily_memory_files()
            .map_err(string_err)
            .and_then(to_value),

        // `{ filename }` -> string | null.
        "read_daily_memory_file" => match str_arg(args, "filename") {
            Ok(filename) => commands::workspace::read_daily_memory_file(filename.to_string())
                .map_err(string_err)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ filename, content }` -> void.
        "write_daily_memory_file" => match (str_arg(args, "filename"), str_arg(args, "content")) {
            (Ok(filename), Ok(content)) => commands::workspace::write_daily_memory_file(
                filename.to_string(),
                content.to_string(),
            )
            .map(|_| Value::Null)
            .map_err(string_err),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ filename }` -> void.
        "delete_daily_memory_file" => match str_arg(args, "filename") {
            Ok(filename) => commands::workspace::delete_daily_memory_file(filename.to_string())
                .map(|_| Value::Null)
                .map_err(string_err),
            Err(e) => Err(e),
        },

        // `{ query }` -> array of DailyMemorySearchResult.
        "search_daily_memory_files" => match str_arg(args, "query") {
            Ok(query) => commands::workspace::search_daily_memory_files(query.to_string())
                .map_err(string_err)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // open_workspace_directory is desktop-only (spawns the OS file opener);
        // it falls through to 501.
        _ => return None,
    })
}

/// Wrap a `String` error from a workspace helper into a [`WebError`].
fn string_err(message: String) -> WebError {
    WebError::Domain(AppError::Message(message))
}
