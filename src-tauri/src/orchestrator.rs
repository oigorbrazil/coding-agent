//! Tauri glue for the delegation core in `orchestrator_core`.
//!
//! The MCP server is hosted in-process over HTTP on the `agent_events` listener, so worker state
//! lives next to the UI instead of in a sidecar.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::cli_resolver;
use crate::orchestrator_core::{Core, Launcher, OrchestrationSettings, Planner};

const JOBS_EVENT: &str = "orchestrator://jobs";
// The skill's playbook.py finds the playbooks next to this policy path (`default_policy_path`).
#[cfg(windows)]
const SKILL_POLICY_FILE: &str = "coding-opus-orchestrate.json";
#[cfg(not(windows))]
const SKILL_POLICY_FILE: &str = "orchestrate.json";
const SKILL_POLICY_ENV: &str = "CODING_OPUS_POLICY";
// `user_playbooks_path` in the skill's playbook.py reads the same name and variable.
const SKILL_PLAYBOOKS_FILE: &str = "coding-opus-playbooks.yaml";
const SKILL_PLAYBOOKS_ENV: &str = "CODING_OPUS_PLAYBOOKS";
const BUNDLED_PLAYBOOKS: &str = include_str!("../assets/skills/coding/playbooks.yaml");
const MAX_PLAYBOOKS_BYTES: usize = 1 << 20;

#[derive(Default)]
pub struct OrchestratorState {
    core: Core,
    prepared: AtomicBool,
}

impl OrchestratorState {
    pub fn core(&self) -> &Core {
        &self.core
    }
}

/// Resolving the launcher lazily keeps a missing Codex install from blocking app start; the
/// failure then surfaces as a delivery on the job that needed it. Resolving it scans PATH, so it
/// happens once rather than on every request.
fn prepare(app: &AppHandle, state: &OrchestratorState) {
    if state.prepared.swap(true, Ordering::SeqCst) {
        return;
    }
    let core = state.core.clone();
    // History outlives the app: what each worker was asked and reported is kept, and Codex keeps
    // the thread itself, so a worker can be started again with its context intact.
    if let Ok(path) = crate::paths::orchestrator_store_path(app) {
        core.set_store(path);
        core.restore();
    }
    let handle = app.clone();
    core.set_observer(Arc::new(move |snapshot: Value| {
        let _ = handle.emit(JOBS_EVENT, snapshot);
    }));

    if let Some(program) = cli_resolver::find_windows_cli_launcher("codex") {
        let mut launcher = Launcher::codex_app_server(PathBuf::from(program));
        #[cfg(windows)]
        launcher.env.push((
            "Path".to_string(),
            crate::orchestrator_core::path_without_store_aliases(&cli_resolver::rebuilt_path()),
        ));
        core.set_launcher(launcher);
    }
    if let Some(program) = cli_resolver::find_windows_cli_launcher("claude") {
        core.set_launcher(Launcher::claude_headless(PathBuf::from(program)));
    }
    core.set_shell_host(Arc::new(crate::orchestrator_shell_host::PtyShellHost::new(
        app.clone(),
    )));
}

pub fn handle_mcp_body(
    app: Option<&AppHandle>,
    state: &OrchestratorState,
    body: &str,
    planner: Option<&str>,
) -> Option<String> {
    if let Some(app) = app {
        prepare(app, state);
    }
    crate::orchestrator_core::handle_mcp_body(&state.core, body, planner)
}

#[tauri::command]
pub fn orchestrator_mcp_config_path(
    app: AppHandle,
    planner_id: String,
    planner_label: String,
    planner_agent: String,
) -> Result<String, String> {
    let state = app.state::<OrchestratorState>();
    prepare(&app, &state);
    state.core.register_planner(Planner {
        id: planner_id.clone(),
        label: planner_label,
        agent: planner_agent,
    });
    let endpoint = crate::agent_events::agent_hooks_endpoint()?;
    let token = crate::agent_events::agent_hooks_token();
    let config = json!({
        "mcpServers": {
            "coding-agent": {
                "type": "http",
                "url": format!("{endpoint}/mcp"),
                "headers": {
                    "X-CodingAgent-Token": token,
                    // One config per terminal, so a request says which session it came from.
                    "X-CodingAgent-Planner": planner_id
                }
            }
        }
    });
    // Namespaced by port, like the agent hooks file: two instances writing one shared path would
    // leave whichever started last owning it, pointing the other one's terminals at the wrong app.
    let port = endpoint.rsplit(':').next().unwrap_or("0");
    let safe: String = planner_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let path =
        std::env::temp_dir().join(format!("coding-agent-orchestrator-mcp-{port}-{safe}.json"));
    let body = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    std::fs::write(&path, body).map_err(|error| format!("write_failed:{error}"))?;
    Ok(path.to_string_lossy().into_owned())
}

impl OrchestratorState {
    /// The board as `orchestrator_jobs` returns it, for the remote control view.
    pub(crate) fn board(&self) -> Value {
        self.core.snapshot()
    }
}

#[tauri::command]
pub fn orchestrator_jobs(state: tauri::State<'_, OrchestratorState>) -> Value {
    state.core.snapshot()
}

#[tauri::command]
pub fn orchestrator_set_concurrency(state: tauri::State<'_, OrchestratorState>, limit: usize) {
    state.core.set_concurrency_limit(limit);
}

fn skill_policy_path() -> Result<PathBuf, String> {
    if let Some(configured) = std::env::var_os(SKILL_POLICY_ENV).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(configured));
    }
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|path| path.join("AppData").join("Roaming"))
        })
        .ok_or_else(|| "orchestrator_skill_policy_home_missing".to_string())?
        .join("CodingOpus");

    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|path| path.join(".config"))
        })
        .ok_or_else(|| "orchestrator_skill_policy_home_missing".to_string())?
        .join("coding-opus");

    Ok(base.join(SKILL_POLICY_FILE))
}

/// The Orchestration settings from Preferences, sent when the app loads and whenever they change.
#[tauri::command]
pub fn orchestrator_apply_settings(
    state: tauri::State<'_, OrchestratorState>,
    settings: OrchestrationSettings,
) -> Result<(), String> {
    state.core.apply_settings(settings);
    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPlaybooks {
    text: String,
    path: String,
    custom: bool,
}

fn skill_playbooks_path() -> Result<PathBuf, String> {
    if let Some(configured) =
        std::env::var_os(SKILL_PLAYBOOKS_ENV).filter(|value| !value.is_empty())
    {
        return Ok(PathBuf::from(configured));
    }
    Ok(skill_policy_path()?.with_file_name(SKILL_PLAYBOOKS_FILE))
}

fn read_playbooks(path: &Path) -> Result<SkillPlaybooks, String> {
    let shown = path.display().to_string();
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(SkillPlaybooks {
            text,
            path: shown,
            custom: true,
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(SkillPlaybooks {
            text: BUNDLED_PLAYBOOKS.to_string(),
            path: shown,
            custom: false,
        }),
        Err(error) => Err(format!("read_failed:{error}")),
    }
}

fn write_playbooks(path: &Path, text: &str) -> Result<SkillPlaybooks, String> {
    if text.trim().is_empty() || text.len() > MAX_PLAYBOOKS_BYTES {
        return Err("invalid_playbooks".to_string());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "orchestrator_playbooks_parent_missing".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| format!("mkdir_failed:{error}"))?;
    let staging = path.with_extension("yaml.tmp");
    std::fs::write(&staging, text).map_err(|error| format!("write_failed:{error}"))?;
    if let Err(error) = std::fs::rename(&staging, path) {
        let _ = std::fs::remove_file(&staging);
        return Err(format!("write_failed:{error}"));
    }
    read_playbooks(path)
}

/// The playbooks the skill plans with: the person's copy, or the bundled file until they save one.
#[tauri::command]
pub fn orchestrator_playbooks() -> Result<SkillPlaybooks, String> {
    read_playbooks(&skill_playbooks_path()?)
}

#[tauri::command]
pub fn orchestrator_save_playbooks(text: String) -> Result<SkillPlaybooks, String> {
    write_playbooks(&skill_playbooks_path()?, &text)
}

/// The Codex models and efforts the Orchestration settings offer for a role.
#[tauri::command]
pub async fn orchestrator_codex_models(
    app: AppHandle,
    state: tauri::State<'_, OrchestratorState>,
) -> Result<Value, String> {
    prepare(&app, &state);
    let core = state.core.clone();
    tauri::async_runtime::spawn_blocking(move || core.list_codex_models())
        .await
        .map_err(|error| error.to_string())?
}

/// Fed by the same usage poll that drives the warning chip, so the planner and the person read the
/// same numbers at the same cadence.
#[tauri::command]
pub fn orchestrator_set_agent_fitness(
    state: tauri::State<'_, OrchestratorState>,
    agent: String,
    snapshot: Value,
) {
    state.core.set_agent_fitness(&agent, snapshot);
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleSetInput {
    pub id: String,
    pub name: String,
    pub text: String,
}

/// The person's sets, pushed from the frontend whenever they change. An empty list is a real
/// choice ("I removed them all") and is stored as such.
#[tauri::command]
pub fn orchestrator_set_rule_sets(
    state: tauri::State<'_, OrchestratorState>,
    sets: Vec<RuleSetInput>,
) {
    state.core.set_rule_sets(
        sets.into_iter()
            .map(|set| crate::orchestrator_core::RuleSet {
                id: set.id,
                name: set.name,
                text: set.text,
            })
            .collect(),
    );
}

/// What ships with Coding Agent, so the editor can show ours and restore one.
#[tauri::command]
pub fn orchestrator_default_rule_sets() -> Value {
    json!(crate::orchestrator_core::default_rule_sets()
        .into_iter()
        .map(|set| json!({ "id": set.id, "name": set.name, "text": set.text }))
        .collect::<Vec<_>>())
}

/// The pane answers a blocked worker directly: the person is already looking at the question.
#[tauri::command]
pub fn orchestrator_answer(
    state: tauri::State<'_, OrchestratorState>,
    job_id: String,
    decision: String,
) -> Result<Value, String> {
    state.core.answer(&job_id, &decision)
}

#[tauri::command]
pub fn orchestrator_job_diff(
    state: tauri::State<'_, OrchestratorState>,
    job_id: String,
) -> Result<String, String> {
    state.core.job_diff(&job_id)
}

/// Interrupts a running worker and settles it as cancelled — the same path `coding_agent_cancel` takes.
#[tauri::command]
pub fn orchestrator_cancel_job(
    state: tauri::State<'_, OrchestratorState>,
    job_id: String,
) -> Value {
    json!({ "cancelled": state.core.cancel_jobs(&[job_id]) })
}

/// The label of the detached board window for one orchestration pane. The frontend reads the pane
/// back from it, and `capabilities/orchestration-window.json` covers exactly `orchestration-*`.
fn orchestration_window_label(terminal_id: &str) -> String {
    let safe: String = terminal_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("orchestration-{safe}")
}

/// Opens an orchestration pane's board in its own window, or brings back the one already open.
/// Async on purpose: building a window from a synchronous command can deadlock on Windows.
#[tauri::command]
pub async fn open_orchestration_window(app: AppHandle, terminal_id: String) -> Result<(), String> {
    let label = orchestration_window_label(&terminal_id);
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        return window.set_focus().map_err(|error| error.to_string());
    }
    tauri::WebviewWindowBuilder::new(&app, &label, tauri::WebviewUrl::App("index.html".into()))
        .title("Coding Agent")
        .inner_size(1180.0, 780.0)
        .min_inner_size(640.0, 420.0)
        .build()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Lets the pane talk to one worker without going through the lead. A worker mid-turn is steered so
/// the correction lands on what it is doing now; an idle one gets the message as a new turn.
#[tauri::command]
pub fn orchestrator_message(
    state: tauri::State<'_, OrchestratorState>,
    job_id: String,
    message: String,
    steer: bool,
) -> Result<Value, String> {
    let mut arguments = serde_json::Map::new();
    arguments.insert("jobId".into(), Value::String(job_id));
    arguments.insert("message".into(), Value::String(message));
    let tool = if steer {
        "coding_agent_steer"
    } else {
        "coding_agent_send"
    };
    crate::orchestrator_core::call_tool(&state.core, tool, &arguments, None)
}

/// The board's shell controls. Stopping waits up to five seconds for Ctrl+C to land, so the work
/// runs off the thread that received the command.
async fn on_shells<F>(app: AppHandle, work: F) -> Result<Value, String>
where
    F: FnOnce(&Core) -> Result<Value, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<OrchestratorState>();
        prepare(&app, &state);
        work(state.core())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub async fn orchestrator_shell_output(
    app: AppHandle,
    shell_id: String,
    lines: usize,
) -> Result<Value, String> {
    on_shells(app, move |core| core.shell_output(&shell_id, lines)).await
}

#[tauri::command]
pub async fn orchestrator_shell_stop(app: AppHandle, shell_id: String) -> Result<Value, String> {
    on_shells(app, move |core| core.stop_shell(&shell_id)).await
}

#[tauri::command]
pub async fn orchestrator_shell_restart(app: AppHandle, shell_id: String) -> Result<Value, String> {
    on_shells(app, move |core| core.restart_shell(&shell_id)).await
}

#[tauri::command]
pub async fn orchestrator_shell_remove(app: AppHandle, shell_id: String) -> Result<Value, String> {
    on_shells(app, move |core| core.remove_shell(&shell_id)).await
}

/// The board's Restart: the same request again as a new worker under the same planner.
#[tauri::command]
pub fn orchestrator_restart(
    state: tauri::State<'_, OrchestratorState>,
    job_id: String,
) -> Result<Value, String> {
    crate::orchestrator_core::restart_job(&state.core, &job_id)
}

#[cfg(test)]
mod tests {
    use super::{orchestration_window_label, read_playbooks, write_playbooks, BUNDLED_PLAYBOOKS};

    // One detached board window per orchestration pane, with a label the capability matches (#247).
    #[test]
    fn a_board_window_label_is_safe_and_matches_its_capability() {
        assert_eq!(
            orchestration_window_label("orchestrator-mWHJe7AX_ilh-tn"),
            "orchestration-orchestrator-mWHJe7AX_ilh-tn"
        );
        assert_eq!(
            orchestration_window_label("../pane id?"),
            "orchestration-___pane_id_"
        );
    }

    fn scratch_playbooks() -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!("coding-opus-playbooks-{}", nanoid::nanoid!(8)))
            .join("coding-opus-playbooks.yaml")
    }

    #[test]
    fn the_bundled_playbooks_stand_in_until_the_person_saves_a_copy() {
        let path = scratch_playbooks();
        let read = read_playbooks(&path).expect("read");
        assert!(!read.custom);
        assert_eq!(read.text, BUNDLED_PLAYBOOKS);

        let saved = write_playbooks(&path, "version: 1\nplaybooks: {}\n").expect("save");
        assert!(saved.custom);
        assert_eq!(saved.text, "version: 1\nplaybooks: {}\n");
        assert!(!path.with_extension("yaml.tmp").exists());

        let again = write_playbooks(&path, "version: 1\nplaybooks: { a: {} }\n").expect("save");
        assert_eq!(again.text, "version: 1\nplaybooks: { a: {} }\n");
        let _ = std::fs::remove_dir_all(path.parent().expect("parent"));
    }

    #[test]
    fn empty_or_oversized_playbooks_are_refused() {
        let path = scratch_playbooks();
        assert_eq!(
            write_playbooks(&path, "  \n").unwrap_err(),
            "invalid_playbooks"
        );
        assert_eq!(
            write_playbooks(&path, &"a".repeat((1 << 20) + 1)).unwrap_err(),
            "invalid_playbooks"
        );
        assert!(!path.exists());
    }
}
