//! Merging ai-memory's lifecycle hooks into the settings file Coding Agent writes per terminal.
//!
//! Coding Agent owns that file, so this is the only place either side's hooks meet: the person's own
//! `settings.json` is never touched, the scope is one terminal, and stopping the integration simply
//! stops writing them.

use serde_json::{Map, Value};

/// Adds ai-memory's hook entries to Coding Agent's hooks object.
///
/// Arrays for a shared event are concatenated rather than replaced, which is how Claude Code treats
/// hooks from several settings sources. All or nothing: a shape we do not recognise leaves `ours`
/// exactly as it was.
pub fn merge_hooks(ours: &mut Map<String, Value>, theirs: &Value) -> Result<(), String> {
    let theirs = theirs
        .as_object()
        .ok_or_else(|| "ai_memory_hooks_not_an_object".to_string())?;

    let mut additions: Vec<(String, Vec<Value>)> = Vec::new();
    for (event, entries) in theirs {
        let entries = entries
            .as_array()
            .ok_or_else(|| format!("ai_memory_hooks_event_not_an_array:{event}"))?;
        additions.push((event.clone(), entries.clone()));
    }

    for (event, entries) in additions {
        match ours.get_mut(&event).and_then(Value::as_array_mut) {
            Some(existing) => existing.extend(entries),
            None => {
                ours.insert(event, Value::Array(entries));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn coding_agent_side() -> serde_json::Map<String, serde_json::Value> {
        // What `agent_hooks_settings_path` already builds: http hooks back to Coding Agent's listener.
        let hook = json!([{ "hooks": [{ "type": "http", "url": "http://127.0.0.1:9123/hook" }] }]);
        let mut hooks = serde_json::Map::new();
        hooks.insert("SessionStart".into(), hook.clone());
        hooks.insert("UserPromptSubmit".into(), hook);
        hooks
    }

    fn their_side() -> serde_json::Value {
        json!({
            "SessionStart": [{ "matcher": "", "hooks": [
                { "type": "command", "command": "ai-memory", "args": ["hook", "--event", "session-start"] }
            ]}],
            "Stop": [{ "matcher": "", "hooks": [
                { "type": "command", "command": "ai-memory", "args": ["hook", "--event", "stop"] }
            ]}]
        })
    }

    #[test]
    fn both_sides_survive_on_an_event_they_share() {
        // Verified against the real CLI before this was designed: hook arrays from two settings
        // sources both fire. Losing either side breaks capture or orchestration, silently.
        let mut ours = coding_agent_side();
        merge_hooks(&mut ours, &their_side()).expect("merges");

        let start = ours.get("SessionStart").unwrap().as_array().unwrap();
        assert_eq!(start.len(), 2, "Coding Agent's entry and theirs: {start:?}");
        let rendered = serde_json::to_string(start).unwrap();
        assert!(rendered.contains("\"type\":\"http\""), "{rendered}");
        assert!(rendered.contains("session-start"), "{rendered}");
    }

    #[test]
    fn an_event_only_ai_memory_wants_is_added() {
        let mut ours = coding_agent_side();
        merge_hooks(&mut ours, &their_side()).unwrap();
        assert_eq!(ours.get("Stop").unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn an_event_only_coding_agent_wants_is_untouched() {
        let mut ours = coding_agent_side();
        merge_hooks(&mut ours, &their_side()).unwrap();
        assert_eq!(ours.get("UserPromptSubmit").unwrap().as_array().unwrap().len(), 1);
    }

    #[test]
    fn an_unfamiliar_shape_is_refused_and_changes_nothing() {
        // Every terminal reads this file. A half-merged one breaks all of them, so a shape we do
        // not recognise must leave Coding Agent's own hooks exactly as they were.
        for bad in [json!("nope"), json!(["SessionStart"]), json!({ "SessionStart": 3 })] {
            let mut ours = coding_agent_side();
            let before = serde_json::to_string(&ours).unwrap();
            assert!(merge_hooks(&mut ours, &bad).is_err(), "{bad:?}");
            assert_eq!(serde_json::to_string(&ours).unwrap(), before, "left intact");
        }
    }
}
