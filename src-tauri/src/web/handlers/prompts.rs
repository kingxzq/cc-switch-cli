//! Prompt commands (`src/lib/api/prompts.ts`).
//!
//! Follows the [`super::providers`] template: each command parses its args and
//! delegates to [`PromptService`], mapping `AppError` -> [`WebError::Domain`].
//! All prompt service fns are synchronous, so no `block_on` is needed.

use serde_json::Value;

use super::common::{app, from_arg, str_arg, to_value};
use crate::prompt::Prompt;
use crate::web::error::WebError;
use crate::{AppState, PromptService};

pub fn dispatch(state: &AppState, command: &str, args: &Value) -> Option<Result<Value, WebError>> {
    Some(match command {
        // `{ app }` -> Record<string, Prompt>. IndexMap serializes to a JSON object.
        "get_prompts" => match app(args) {
            Ok(t) => PromptService::get_prompts(state, t)
                .map_err(WebError::Domain)
                .and_then(to_value),
            Err(e) => Err(e),
        },

        // `{ app, id, prompt }` -> void.
        "upsert_prompt" => match (
            app(args),
            str_arg(args, "id"),
            from_arg::<Prompt>(args, "prompt"),
        ) {
            (Ok(t), Ok(id), Ok(prompt)) => PromptService::upsert_prompt(state, t, id, prompt)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => Err(e),
        },

        // `{ app, id }` -> void.
        "delete_prompt" => match (app(args), str_arg(args, "id")) {
            (Ok(t), Ok(id)) => PromptService::delete_prompt(state, t, id)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ app, id }` -> void.
        "enable_prompt" => match (app(args), str_arg(args, "id")) {
            (Ok(t), Ok(id)) => PromptService::enable_prompt(state, t, id)
                .map(|_| Value::Null)
                .map_err(WebError::Domain),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },

        // `{ app }` -> string (the new prompt id).
        "import_prompt_from_file" => match app(args) {
            Ok(t) => PromptService::import_from_file(state, t)
                .map(Value::String)
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        // `{ app }` -> string | null (None when the live file is absent).
        "get_current_prompt_file_content" => match app(args) {
            Ok(t) => PromptService::get_current_file_content(t)
                .map(|opt| opt.map(Value::String).unwrap_or(Value::Null))
                .map_err(WebError::Domain),
            Err(e) => Err(e),
        },

        _ => return None,
    })
}
