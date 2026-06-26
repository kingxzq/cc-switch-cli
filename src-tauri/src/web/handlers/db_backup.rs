//! Database-backup and Codex unified-history-backup commands
//! (`src/lib/api/settings.ts`: `backupsApi.*`, `settingsApi.*HistoryBackup`).
//!
//! Follows the [`super::meta`] template. Only the commands with a confirmed
//! cc-switch-cli backing fn are wired here:
//!
//!   - `create_db_backup` -> `Database::backup_database_file`
//!   - `has_codex_unify_history_backup` ->
//!     `codex_history_migration::has_codex_official_history_unify_backup`
//!   - `restore_codex_unified_history` ->
//!     `codex_history_migration::restore_codex_official_history_from_backups`
//!
//! The named-file DB backup CRUD (`list_db_backups`, `restore_db_backup`,
//! `rename_db_backup`, `delete_db_backup`) is NOT wired: cc-switch-cli has no
//! `Database::list_backups` / `restore_from_backup` / `rename_backup` /
//! `delete_backup` and no `BackupEntry` type. (`ConfigService::list_backups`
//! is unrelated — it lists live config-file backups by `config_path` and
//! returns `BackupInfo`.) Those commands fall through to HTTP 501.

use serde_json::{json, Value};

use crate::web::error::WebError;
use crate::AppState;

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    // These commands take no arguments.
    let _ = args;
    Some(match command {
        // No args -> string (backup filename). `backup_database_file` returns
        // `Result<Option<PathBuf>, AppError>`; None means the DB file is missing.
        "create_db_backup" => match state.db.backup_database_file() {
            Ok(Some(path)) => Ok(Value::String(
                path.file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )),
            Ok(None) => Err(WebError::Domain(crate::AppError::Config(
                "Database file not found, backup skipped".to_string(),
            ))),
            Err(e) => Err(WebError::Domain(e)),
        },

        // No args -> bool. Infallible (returns bool directly).
        "has_codex_unify_history_backup" => Ok(Value::Bool(
            crate::codex_history_migration::has_codex_official_history_unify_backup(),
        )),

        // No args -> CodexUnifyHistoryRestoreResult. The outcome struct does not
        // derive Serialize and is snake_case, so build the camelCase TS shape
        // ({ restoredJsonlFiles, restoredStateRows, skippedReason? }) by hand.
        "restore_codex_unified_history" => {
            match crate::codex_history_migration::restore_codex_official_history_from_backups() {
                Ok(outcome) => {
                    let mut value = json!({
                        "restoredJsonlFiles": outcome.restored_jsonl_files,
                        "restoredStateRows": outcome.restored_state_rows,
                    });
                    if let Some(reason) = outcome.skipped_reason {
                        value["skippedReason"] = Value::String(reason);
                    }
                    Ok(value)
                }
                Err(e) => Err(WebError::Domain(e)),
            }
        }

        _ => return None,
    })
}
