//! Per-domain command handlers for the web invoke bridge.
//!
//! Each submodule exposes `dispatch(state, command, args) -> Option<Result<
//! Value, WebError>>`, returning `None` when the command does not belong to it.
//! The central [`dispatch`] tries each module in turn and falls back to
//! `NotImplemented` (HTTP 501) so unwired commands degrade gracefully.
//!
//! See [`meta`] for the reference template every module follows.

use serde_json::Value;

use crate::web::error::WebError;
use crate::AppState;

pub mod common;

pub mod auth;
pub mod config;
pub mod config_misc;
pub mod copilot;
pub mod db_backup;
pub mod deeplink;
pub mod env;
pub mod failover;
pub mod global_proxy;
pub mod hermes;
pub mod mcp;
pub mod mcp_config;
pub mod meta;
pub mod model_fetch;
pub mod model_test;
pub mod omo;
pub mod openclaw;
pub mod pricing;
pub mod prompts;
pub mod providers;
pub mod providers_extra;
pub mod proxy;
pub mod proxy_extra;
pub mod sessions;
pub mod settings;
pub mod skills;
pub mod subscription;
pub mod usage;
pub mod vscode;
pub mod workspace;

/// Signature shared by every module dispatcher.
type ModuleDispatch = fn(&AppState, &str, &Value) -> Option<Result<Value, WebError>>;

/// The dispatch chain. Command names are globally unique across modules, so the
/// order only affects which module "claims" a name first (no ambiguity).
const MODULES: &[ModuleDispatch] = &[
    meta::dispatch,
    providers::dispatch,
    providers_extra::dispatch,
    auth::dispatch,
    config::dispatch,
    config_misc::dispatch,
    copilot::dispatch,
    db_backup::dispatch,
    deeplink::dispatch,
    env::dispatch,
    failover::dispatch,
    global_proxy::dispatch,
    hermes::dispatch,
    mcp::dispatch,
    mcp_config::dispatch,
    model_fetch::dispatch,
    model_test::dispatch,
    omo::dispatch,
    openclaw::dispatch,
    pricing::dispatch,
    prompts::dispatch,
    proxy::dispatch,
    proxy_extra::dispatch,
    sessions::dispatch,
    settings::dispatch,
    skills::dispatch,
    subscription::dispatch,
    usage::dispatch,
    vscode::dispatch,
    workspace::dispatch,
];

/// Route a command to the first module that handles it, else 501.
pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Result<Value, WebError> {
    for module in MODULES {
        if let Some(result) = module(state, command, args) {
            return result;
        }
    }
    Err(WebError::NotImplemented(command.to_string()))
}
