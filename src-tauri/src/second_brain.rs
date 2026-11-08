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
const MAX_TITLE_CHARS: usize = 120;
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

fn yaml_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn timestamp_parts(now: SystemTime) -> (String, String) {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let moment = chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default();
    (
        moment.format("%Y%m%d-%H%M").to_string(),
        moment.format("%Y-%m-%d").to_string(),
    )
}

pub fn render_note(request: &SecondBrainRecordRequest, now: SystemTime) -> String {
    let (id, date) = timestamp_parts(now);
    let mut tags: Vec<String> = vec!["coding-agent".to_string()];
    for tag in &request.tags {
        let tag = slugify(tag);
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    let title: String = request.title.trim().chars().take(MAX_TITLE_CHARS).collect();
    let mut note = String::new();
    note.push_str("---\n");
    note.push_str(&format!("id: {}\n", yaml_string(&id)));
    note.push_str(&format!("title: {}\n", yaml_string(&title)));
    note.push_str("area: projetos\n");
    note.push_str("type: nota\n");
    note.push_str("tags:\n");
    for tag in &tags {
        note.push_str(&format!("  - {tag}\n"));
    }
    note.push_str("status: inbox\n");
    note.push_str(&format!("created: {date}\n"));
    note.push_str(&format!("updated: {date}\n"));
    note.push_str("---\n\n");
    note.push_str(&format!("# {title}\n\n"));
    if !request.summary.trim().is_empty() {
        note.push_str(request.summary.trim());
        note.push_str("\n\n");
    }
    if !request.body.trim().is_empty() {
        note.push_str(request.body.trim());
        note.push('\n');
    }
    note
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
    let content = render_note(request, now);
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

    #[test]
    fn record_writes_frontmatter_and_never_overwrites() {
        let root = temp_root("record");
        let now = UNIX_EPOCH + Duration::from_secs(1_794_096_000);
        let first = record_at(&root, &request("Lote de teste"), now).unwrap();
        let second = record_at(&root, &request("Lote de teste"), now).unwrap();
        assert!(
            first.ends_with("00-Inbox/lote-de-teste.md")
                || first.ends_with("00-Inbox\\lote-de-teste.md")
        );
        assert_ne!(first, second);
        let text = fs::read_to_string(&first).unwrap();
        assert!(text.starts_with("---\nid: \""));
        assert!(text.contains("title: \"Lote de teste\""));
        assert!(text.contains("  - coding-agent\n  - projeto-x\n"));
        assert!(text.contains("status: inbox"));
        assert!(text.contains("# Lote de teste\n\nResumo\n\nCorpo\n"));
        fs::remove_dir_all(root).ok();
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
