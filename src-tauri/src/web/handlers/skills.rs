//! Skill + skill-repo commands (`src/lib/api/skills.ts`).
//!
//! Follows the [`super::meta`] / [`super::providers`] templates. Maps to the
//! free-standing `SkillService` (no `&AppState` needed; skills persist in their
//! own SQLite tables + SSOT dir). Async service fns are bridged with
//! [`super::common::block_on`].
//!
//! Desktop-only commands (native ZIP file dialog) and features the CLI does not
//! implement yet (backups, update checks, storage migration, skills.sh search)
//! are intentionally left unwired and fall through to HTTP 501.

use serde_json::{json, Value};

use super::common::{app_arg, block_on, from_arg, str_arg, to_value};
use crate::services::skill::{DiscoverableSkill, ImportSkillSelection, SkillRepo};
use crate::web::error::WebError;
use crate::{AppState, AppType, SkillService};

pub fn dispatch(_state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // ---- Unified management API (v3.10.0+) ----

        // No args -> InstalledSkill[].
        "get_installed_skills" => SkillService::list_installed()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ skill: DiscoverableSkill, currentApp }` -> InstalledSkill.
        // `install` resolves the spec via repo discovery; the skill `key`
        // (owner/name:directory) is the unambiguous spec.
        "install_skill_unified" => {
            match (
                from_arg::<DiscoverableSkill>(args, "skill"),
                app_arg(args, "currentApp"),
            ) {
                (Ok(skill), Ok(app)) => match SkillService::new() {
                    Ok(service) => block_on(service.install(&skill.key, &app))
                        .map_err(WebError::Domain)
                        .and_then(to_value),
                    Err(e) => Err(WebError::Domain(e)),
                },
                (Err(e), _) | (_, Err(e)) => Err(e),
            }
        }

        // `{ id }` -> SkillUninstallResult { backupPath? }. The CLI uninstall
        // has no backup step, so return an empty result object.
        "uninstall_skill_unified" => match str_arg(args, "id") {
            Ok(id) => SkillService::uninstall(id)
                .map(|_| json!({}))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ id, app, enabled }` -> bool.
        "toggle_skill_app" => match (str_arg(args, "id"), app_arg(args, "app")) {
            (Ok(id), Ok(app)) => {
                let enabled = super::common::bool_arg(args, "enabled", false);
                SkillService::toggle_app(id, &app, enabled)
                    .map(|_| Value::Bool(true))
                    .map_err(WebError::Domain)
            }
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // No args -> UnmanagedSkill[].
        "scan_unmanaged_skills" => SkillService::scan_unmanaged()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ imports: ImportSkillSelection[] }` -> InstalledSkill[].
        "import_skills_from_apps" => match from_arg::<Vec<ImportSkillSelection>>(args, "imports") {
            Ok(imports) => SkillService::import_from_apps(imports)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // No args -> DiscoverableSkill[] (across all enabled repos).
        "discover_available_skills" => match SkillService::load_index() {
            Ok(index) => match SkillService::new() {
                Ok(service) => block_on(service.discover_available(index.repos))
                    .map_err(WebError::Domain)
                    .and_then(to_value),
                Err(e) => Err(WebError::Domain(e)),
            },
            Err(e) => Err(WebError::Domain(e)),
        },

        // ---- Legacy compat API ----

        // No args -> Skill[] (discoverable + installed flag).
        "get_skills" => match SkillService::new() {
            Ok(service) => block_on(service.list_skills())
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(WebError::Domain(e)),
        },

        // `{ app }` -> Skill[]. The CLI listing is not app-scoped; the app only
        // affects the frontend's installed badge, so reuse `list_skills`.
        "get_skills_for_app" => match app_arg(args, "app") {
            Ok(_app) => match SkillService::new() {
                Ok(service) => block_on(service.list_skills())
                    .map_err(WebError::Domain)
                    .and_then(to_value),
                Err(e) => Err(WebError::Domain(e)),
            },
            Err(e) => Err(e),
        },

        // `{ directory }` -> bool. Claude-only install.
        "install_skill" => match str_arg(args, "directory") {
            Ok(directory) => match SkillService::new() {
                Ok(service) => block_on(service.install(directory, &AppType::Claude))
                    .map(|_| Value::Bool(true))
                    .map_err(WebError::Domain),
                Err(e) => Err(WebError::Domain(e)),
            },
            Err(e) => Err(e),
        },

        // `{ app, directory }` -> bool.
        "install_skill_for_app" => match (app_arg(args, "app"), str_arg(args, "directory")) {
            (Ok(app), Ok(directory)) => match SkillService::new() {
                Ok(service) => block_on(service.install(directory, &app))
                    .map(|_| Value::Bool(true))
                    .map_err(WebError::Domain),
                Err(e) => Err(WebError::Domain(e)),
            },
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ directory }` -> SkillUninstallResult. Claude-only -> global uninstall.
        "uninstall_skill" => match str_arg(args, "directory") {
            Ok(directory) => SkillService::uninstall(directory)
                .map(|_| json!({}))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ app, directory }` -> SkillUninstallResult. Per-app removal = disable
        // for that app (removes the skill from that app's dir).
        "uninstall_skill_for_app" => match (app_arg(args, "app"), str_arg(args, "directory")) {
            (Ok(app), Ok(directory)) => SkillService::toggle_app(directory, &app, false)
                .map(|_| json!({}))
                .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // ---- Repo management ----

        // No args -> SkillRepo[].
        "get_skill_repos" => SkillService::list_repos()
            .map_err(WebError::Domain)
            .and_then(to_value),

        // `{ repo: SkillRepo }` -> bool.
        "add_skill_repo" => match from_arg::<SkillRepo>(args, "repo") {
            Ok(repo) => SkillService::upsert_repo(repo)
                .map(|_| Value::Bool(true))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ owner, name }` -> bool.
        "remove_skill_repo" => match (str_arg(args, "owner"), str_arg(args, "name")) {
            (Ok(owner), Ok(name)) => SkillService::remove_repo(owner, name)
                .map(|_| Value::Bool(true))
                .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // Unwired (no backing fn / desktop-only), fall through to 501:
        //   get_skill_backups, delete_skill_backup, restore_skill_backup,
        //   check_skill_updates, update_skill, migrate_skill_storage,
        //   search_skills_sh, open_zip_file_dialog, install_skills_from_zip.
        _ => return None,
    })
}
