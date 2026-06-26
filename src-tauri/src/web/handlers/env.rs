//! Environment-variable commands (`src/lib/api/env.ts`).
//!
//! Follows the [`super::meta`] template. Covers env-conflict checks, deletion
//! with backup, and backup restore. The frontend's `checkAllEnvConflicts` is a
//! pure client-side fan-out over `check_env_conflicts`, so there is no backend
//! command to wire for it.

use serde_json::Value;

use super::common::{app, from_arg, ok_null, str_arg, to_value};
use crate::services::env_checker::{self, EnvConflict};
use crate::services::env_manager;
use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ app }` arg -> EnvConflict[]. The service takes the app label as &str.
        "check_env_conflicts" => match app(args) {
            Ok(app_type) => env_checker::check_env_conflicts(app_type.as_str())
                .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ conflicts: EnvConflict[] }` arg -> BackupInfo object.
        "delete_env_vars" => match from_arg::<Vec<EnvConflict>>(args, "conflicts") {
            Ok(conflicts) => env_manager::delete_env_vars(conflicts)
                .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ backupPath: string }` arg -> void.
        "restore_env_backup" => match str_arg(args, "backupPath") {
            Ok(path) => env_manager::restore_from_backup(path.to_string())
                .map(|_| ())
                .map_err(|e| WebError::Domain(crate::AppError::Message(e)))
                .and_then(|_| ok_null()),
            Err(e) => Err(e),
        },

        // `checkAllEnvConflicts` is a frontend-only fan-out over
        // `check_env_conflicts`; no dedicated backend command exists.
        _ => return None,
    })
}
