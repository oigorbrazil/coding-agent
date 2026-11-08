//! Second brain: the global Markdown notes folder the bundled Coding Opus skill reads and writes.
//!
//! The folder lives at `<Documents>/second brain` unless `CODING_OPUS_BRAIN` points elsewhere,
//! which is the same rule as the skill's `brain.py`. The app only reports on it and appends new
//! notes on an explicit request; it never edits or deletes an existing note.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const FOLDER_NAME: &str = "second brain";
const BRAIN_ENV: &str = "CODING_OPUS_BRAIN";
const DEFAULT_FOLDER: &str = "00-Inbox";
const TEMPLATES_FOLDER: &str = "99-Templates";
const RECENT_LIMIT: usize = 5;
const MAX_SCAN_DEPTH: usize = 6;
const MAX_NOTE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondBrainNote {
    /// Path relative to the second brain root, with forward slashes.
    pub path: String,
    pub title: String,
    pub modified_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondBrainStatus {
    pub path: String,
    pub exists: bool,
    /// True when `CODING_OPUS_BRAIN` chose the folder instead of the Documents default.
    pub from_env: bool,
    pub note_count: usize,
    pub folders: Vec<String>,
    pub recent: Vec<SecondBrainNote>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecondBrainRecordRequest {
    pub title: String,
    #[serde(default)]
    pub folder: Option<String>,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// `nota` unless the caller says otherwise; `decisao` adds the Motivo/Consequências sections.
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub area: String,
    /// Wikilink targets listed under "Notas relacionadas".
    #[serde(default)]
    pub related: Vec<String>,
    /// The caller's local time as `YYYYMMDD-HHMM`; the note id and dates come from it, as the
    /// skill uses local time. Falls back to UTC.
    #[serde(default)]
    pub local_stamp: Option<String>,
}

fn documents_dir() -> Option<PathBuf> {
    dirs_next::document_dir().or_else(|| dirs_next::home_dir().map(|home| home.join("Documents")))
}

/// The folder the skill uses: the env override, else a case-insensitive "second brain" in Documents.
pub fn brain_root() -> Option<(PathBuf, bool)> {
    if let Some(configured) = std::env::var_os(BRAIN_ENV).filter(|value| !value.is_empty()) {
        return Some((PathBuf::from(configured), true));
    }
    let documents = documents_dir()?;
    if let Ok(entries) = fs::read_dir(&documents) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().eq_ignore_ascii_case(FOLDER_NAME)
                && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
            {
                return Some((entry.path(), false));
            }
        }
    }
    Some((documents.join(FOLDER_NAME), false))
}

fn modified_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// The `title:` from a note's frontmatter, else its file name.
fn note_title(path: &Path) -> String {
    let fallback = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    let Ok(text) = fs::read_to_string(path) else {
        return fallback;
    };
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("---") {
        return fallback;
    }
    for line in lines.take(40) {
        let line = line.trim();
        if line == "---" {
            break;
        }
        if let Some(value) = line.strip_prefix("title:") {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !value.is_empty() {
                return value.to_string();
            }
        }
    }
    fallback
}

fn collect_notes(root: &Path, dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_SCAN_DEPTH {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            if depth == 0 && name == TEMPLATES_FOLDER {
                continue;
            }
            collect_notes(root, &path, depth + 1, out);
        } else if name.to_ascii_lowercase().ends_with(".md") && !(depth == 0 && name == "HOME.md") {
            out.push(path);
        }
    }
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn status_at(root: &Path, from_env: bool) -> SecondBrainStatus {
    let exists = root.is_dir();
    let mut folders = Vec::new();
    let mut notes = Vec::new();
    if exists {
        if let Ok(entries) = fs::read_dir(root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if !name.starts_with('.') && entry.file_type().map(|k| k.is_dir()).unwrap_or(false)
                {
                    folders.push(name);
                }
            }
        }
        folders.sort();
        collect_notes(root, root, 0, &mut notes);
    }
    let mut dated: Vec<(u64, PathBuf)> = notes
        .iter()
        .map(|path| (modified_ms(path), path.clone()))
        .collect();
    dated.sort_by(|a, b| b.0.cmp(&a.0));
    let recent = dated
        .into_iter()
        .take(RECENT_LIMIT)
        .map(|(modified_ms, path)| SecondBrainNote {
            path: relative(root, &path),
            title: note_title(&path),
            modified_ms,
        })
        .collect();
    SecondBrainStatus {
        path: root.to_string_lossy().into_owned(),
        exists,
        from_env,
        note_count: notes.len(),
        folders,
        recent,
    }
}

/// Lowercase ASCII kebab-case without accents, as the skill names its notes.
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars().flat_map(char::to_lowercase) {
        let mapped = match ch {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if mapped == '-' {
            if !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
        } else {
            out.push(mapped);
        }
    }
    let trimmed = out.trim_matches('-');
    let limited: String = trimmed.chars().take(80).collect();
    let limited = limited.trim_end_matches('-').to_string();
    if limited.is_empty() {
        "nota".to_string()
    } else {
        limited
    }
}

/// A folder relative to the root with no `..`, drive or absolute part.
fn safe_folder(folder: &str) -> Result<PathBuf, String> {
    let folder = folder.trim().trim_matches(|c| c == '/' || c == '\\');
    if folder.is_empty() {
        return Ok(PathBuf::from(DEFAULT_FOLDER));
    }
    if folder.contains(':') {
        return Err("second_brain_folder_invalid".to_string());
    }
    let path = PathBuf::from(folder.replace('\\', "/"));
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("second_brain_folder_invalid".to_string());
    }
    Ok(path)
}

const NOTE_KINDS: [&str; 5] = ["decisao", "nota", "estudo", "ideia", "meeting"];
const SUMMARY_PLACEHOLDER: &str = "_Resumo do lead: o que foi decidido ou entregue e por quê._";

/// A JSON string, as `json.dumps(..., ensure_ascii=False)` writes it in brain.py.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

fn stamp_of(request: &SecondBrainRecordRequest, now: SystemTime) -> chrono::NaiveDateTime {
    if let Some(stamp) = request.local_stamp.as_deref() {
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(stamp, "%Y%m%d-%H%M") {
            return parsed;
        }
    }
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    chrono::DateTime::from_timestamp(secs, 0)
        .unwrap_or_default()
        .naive_utc()
}

fn existing_ids(root: &Path) -> std::collections::HashSet<String> {
    let mut notes = Vec::new();
    collect_notes(root, root, 0, &mut notes);
    let mut ids = std::collections::HashSet::new();
    for path in notes {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("---") {
            continue;
        }
        for line in lines.take(40) {
            let line = line.trim();
            if line == "---" {
                break;
            }
            if let Some(value) = line.strip_prefix("id:") {
                ids.insert(
                    value
                        .trim()
                        .trim_matches(|c| c == '"' || c == '\'')
                        .to_string(),
                );
            }
        }
    }
    ids
}

/// The first free `YYYYMMDD-HHMM` from `stamp` on, one minute at a time, like brain.py's `new_id`.
fn unique_id(stamp: chrono::NaiveDateTime, taken: &std::collections::HashSet<String>) -> String {
    let mut current = stamp;
    for _ in 0..1440 {
        let id = current.format("%Y%m%d-%H%M").to_string();
        if !taken.contains(&id) {
            return id;
        }
        current += chrono::Duration::minutes(1);
    }
    current.format("%Y%m%d-%H%M").to_string()
}

/// The single Coding Opus note format. brain.py's `render_note` writes the same text.
pub fn render_note(
    request: &SecondBrainRecordRequest,
    folder: &str,
    id: &str,
    date: &str,
) -> String {
    let kind = request
        .kind
        .as_deref()
        .filter(|kind| NOTE_KINDS.contains(kind))
        .unwrap_or("nota");
    let mut tags = vec!["coding-agent".to_string(), "orquestracao".to_string()];
    for tag in request.tags.iter().filter(|tag| !tag.trim().is_empty()) {
        let tag = slugify(tag);
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    let title = request.title.trim();
    let first = folder.split('/').next().unwrap_or("");
    let status = if first == DEFAULT_FOLDER {
        "inbox"
    } else {
        "ativo"
    };
    let summary = request.summary.trim();
    let mut lines: Vec<String> = vec![
        "---".into(),
        format!("id: {}", quoted(id)),
        format!("title: {}", quoted(title)),
        format!("area: {}", quoted(request.area.trim())),
        format!("type: {kind}"),
        format!("tags: [{}]", tags.join(", ")),
        format!("status: {status}"),
        format!("created: {}", quoted(date)),
        format!("updated: {}", quoted(date)),
        "---".into(),
        String::new(),
        format!("# {title}"),
        String::new(),
        format!(
            "## {}",
            if kind == "decisao" {
                "Decisão"
            } else {
                "Conteúdo"
            }
        ),
        String::new(),
        if summary.is_empty() {
            SUMMARY_PLACEHOLDER.to_string()
        } else {
            summary.to_string()
        },
        String::new(),
    ];
    if !request.body.trim().is_empty() {
        lines.push(request.body.trim().to_string());
        lines.push(String::new());
    }
    if kind == "decisao" {
        lines.extend([
            "## Motivo".into(),
            String::new(),
            "## Consequências".into(),
            String::new(),
        ]);
    }
    lines.push("## Notas relacionadas".into());
    lines.push(String::new());
    lines.extend(request.related.iter().map(|note| format!("- [[{note}]]")));
    lines.push(String::new());
    lines.join("\n")
}

/// Writes a new note under `root/<folder>`, never replacing an existing file.
pub fn record_at(
    root: &Path,
    request: &SecondBrainRecordRequest,
    now: SystemTime,
) -> Result<PathBuf, String> {
    if request.title.trim().is_empty() {
        return Err("second_brain_title_missing".to_string());
    }
    if !root.is_dir() {
        return Err("second_brain_missing".to_string());
    }
    let folder = safe_folder(request.folder.as_deref().unwrap_or(DEFAULT_FOLDER))?;
    let stamp = stamp_of(request, now);
    let id = unique_id(stamp, &existing_ids(root));
    let folder_text = folder
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/");
    let content = render_note(
        request,
        &folder_text,
        &id,
        &stamp.format("%Y-%m-%d").to_string(),
    );
    if content.len() > MAX_NOTE_BYTES {
        return Err("second_brain_note_too_large".to_string());
    }
    let dir = root.join(&folder);
    fs::create_dir_all(&dir).map_err(|error| format!("second_brain_write_failed:{error}"))?;
    let stem = slugify(&request.title);
    for attempt in 1..1000 {
        let name = if attempt == 1 {
            format!("{stem}.md")
        } else {
            format!("{stem}-{attempt}.md")
        };
        let target = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(content.as_bytes())
                    .map_err(|error| format!("second_brain_write_failed:{error}"))?;
                return Ok(target);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("second_brain_write_failed:{error}")),
        }
    }
    Err("second_brain_name_exhausted".to_string())
}

#[tauri::command]
pub fn second_brain_status() -> Result<SecondBrainStatus, String> {
    let (root, from_env) =
        brain_root().ok_or_else(|| "second_brain_documents_missing".to_string())?;
    Ok(status_at(&root, from_env))
}

#[tauri::command]
pub fn second_brain_record(request: SecondBrainRecordRequest) -> Result<String, String> {
    let (root, _) = brain_root().ok_or_else(|| "second_brain_documents_missing".to_string())?;
    record_at(&root, &request, SystemTime::now()).map(|path| path.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn temp_root(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("coding-agent-brain-{label}-{}", nanoid::nanoid!(8)));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn request(title: &str) -> SecondBrainRecordRequest {
        SecondBrainRecordRequest {
            title: title.to_string(),
            folder: None,
            summary: "Resumo".to_string(),
            body: "Corpo".to_string(),
            tags: vec!["Projeto X".to_string()],
            kind: None,
            area: String::new(),
            related: Vec::new(),
            local_stamp: Some("20261107-2105".to_string()),
        }
    }

    #[test]
    fn slugify_strips_accents_and_punctuation() {
        assert_eq!(
            slugify("Decisão: usar Codex às 10h!"),
            "decisao-usar-codex-as-10h"
        );
        assert_eq!(slugify("***"), "nota");
    }

    #[test]
    fn status_counts_notes_and_skips_templates_and_home() {
        let root = temp_root("status");
        fs::create_dir_all(root.join("00-Inbox")).unwrap();
        fs::create_dir_all(root.join("99-Templates")).unwrap();
        fs::write(root.join("HOME.md"), "home").unwrap();
        fs::write(root.join("99-Templates/_nota.md"), "t").unwrap();
        fs::write(root.join("00-Inbox/a.md"), "---\ntitle: Nota A\n---\nx").unwrap();
        let status = status_at(&root, false);
        assert!(status.exists);
        assert_eq!(status.note_count, 1);
        assert_eq!(status.recent[0].title, "Nota A");
        assert_eq!(status.recent[0].path, "00-Inbox/a.md");
        assert_eq!(status.folders, vec!["00-Inbox", "99-Templates"]);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_missing_folder_reports_nothing() {
        let root =
            std::env::temp_dir().join(format!("coding-agent-brain-missing-{}", nanoid::nanoid!(8)));
        let status = status_at(&root, true);
        assert!(!status.exists);
        assert_eq!(status.note_count, 0);
        assert!(status.from_env);
    }

    /// Byte-for-byte what `brain.py record --title 'Lote "x"' --type nota --summary Resumo
    /// --body Corpo --tag "Projeto X" --now 2026-11-07T21:05` writes into 00-Inbox.
    const BRAIN_PY_NOTE: &str = "---\nid: \"20261107-2105\"\ntitle: \"Lote \\\"x\\\"\"\narea: \"\"\ntype: nota\ntags: [coding-agent, orquestracao, projeto-x]\nstatus: inbox\ncreated: \"2026-11-07\"\nupdated: \"2026-11-07\"\n---\n\n# Lote \"x\"\n\n## Conteúdo\n\nResumo\n\nCorpo\n\n## Notas relacionadas\n\n";

    #[test]
    fn record_writes_the_same_note_as_brain_py_and_never_overwrites() {
        let root = temp_root("record");
        let first = record_at(&root, &request("Lote \"x\""), SystemTime::now()).unwrap();
        let second = record_at(&root, &request("Lote \"x\""), SystemTime::now()).unwrap();
        assert!(
            first.ends_with("00-Inbox/lote-x.md") || first.ends_with(r"00-Inbox\lote-x.md"),
            "{first:?}"
        );
        assert_ne!(first, second);
        assert_eq!(fs::read_to_string(&first).unwrap(), BRAIN_PY_NOTE);
        // Same minute taken by the first note, so the second moves on one minute like new_id.
        assert!(fs::read_to_string(&second)
            .unwrap()
            .starts_with("---\nid: \"20261107-2106\""));
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_decision_gets_its_sections_and_a_project_folder_is_active() {
        let mut req = request("Usar Codex");
        req.kind = Some("decisao".to_string());
        req.related = vec!["outra-nota".to_string()];
        let note = render_note(&req, "30-Projetos/shop", "20261107-2105", "2026-11-07");
        assert!(note.contains("status: ativo\n"));
        assert!(note.contains("## Decisão\n\nResumo\n\nCorpo\n\n## Motivo\n\n## Consequências\n\n"));
        assert!(note.ends_with("## Notas relacionadas\n\n- [[outra-nota]]\n"));
    }

    #[test]
    fn record_refuses_folders_outside_the_root() {
        let root = temp_root("escape");
        // A leading slash is trimmed, so "/abs" lands inside the root; only escapes are refused.
        for folder in ["../fora", "C:\\x", "a/../../b", ".."] {
            let mut req = request("x");
            req.folder = Some(folder.to_string());
            assert!(
                record_at(&root, &req, SystemTime::now()).is_err(),
                "{folder}"
            );
        }
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn record_needs_an_existing_brain_and_a_title() {
        let root =
            std::env::temp_dir().join(format!("coding-agent-brain-none-{}", nanoid::nanoid!(8)));
        assert_eq!(
            record_at(&root, &request("x"), SystemTime::now()).unwrap_err(),
            "second_brain_missing"
        );
        let existing = temp_root("title");
        assert_eq!(
            record_at(&existing, &request("  "), SystemTime::now()).unwrap_err(),
            "second_brain_title_missing"
        );
        fs::remove_dir_all(existing).ok();
    }
}
