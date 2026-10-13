// Copyright 2026 The Sashiko Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Multi-tree kernel prompt snapshot resolution and deterministic three-part
//! prompt matching (Part 1: Maintainers Notes, Part 2: Code Contexts, Part 3:
//! Kernel API Guides).

use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::baseline::extract_files_from_diff;
use crate::git_cmd;
use crate::maintainers::{CompiledPattern, MaintainersIndex};
use crate::workflows::guard::sanitize_prompt_relpath;

/// Default inline byte budget for Part 1 Maintainers Notes (24 KB).
pub const MAX_MAINTAINER_NOTES_BYTES: usize = 24 * 1024;
/// Default inline byte budget for Part 2 Subsystem/File Code Contexts (48 KB).
pub const MAX_CODE_CONTEXTS_BYTES: usize = 48 * 1024;
/// Default inline byte budget for Part 3 Kernel API Guides (48 KB).
pub const MAX_API_GUIDES_BYTES: usize = 48 * 1024;

/// Major and minor kernel release numbers (`VERSION.PATCHLEVEL`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelVersion {
    pub major: u32,
    pub minor: u32,
}

impl KernelVersion {
    /// Parses a `"<major>.<minor>"` or `"linux-<major>.<minor>.y"` version string.
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw
            .trim()
            .strip_prefix("linux-")
            .unwrap_or(raw.trim())
            .strip_suffix(".y")
            .unwrap_or(raw.trim().strip_prefix("linux-").unwrap_or(raw.trim()));
        let mut parts = trimmed.split('.');
        let major = parts.next()?.trim().parse::<u32>().ok()?;
        let minor_part = parts.next()?.trim();
        let minor_digits: String = minor_part
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let minor = minor_digits.parse::<u32>().ok()?;
        Some(Self { major, minor })
    }

    /// Extracts `VERSION` and `PATCHLEVEL` from the top of a kernel `Makefile`.
    pub fn parse_makefile(content: &str) -> Option<Self> {
        let mut version: Option<u32> = None;
        let mut patchlevel: Option<u32> = None;
        for line in content.lines().take(30) {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("VERSION")
                && let Some(val) = parse_makefile_assignment(rest)
            {
                version = Some(val);
            } else if let Some(rest) = trimmed.strip_prefix("PATCHLEVEL")
                && let Some(val) = parse_makefile_assignment(rest)
            {
                patchlevel = Some(val);
            }
            if let (Some(major), Some(minor)) = (version, patchlevel) {
                return Some(Self { major, minor });
            }
        }
        None
    }
}

fn parse_makefile_assignment(rest: &str) -> Option<u32> {
    let after_eq = rest.trim_start().strip_prefix('=')?;
    after_eq.trim().parse::<u32>().ok()
}

impl fmt::Display for KernelVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// Metadata stored in `trees/<tree_id>/tree_meta.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeMeta {
    pub tree_id: String,
    pub kernel_version: String,
    #[serde(default)]
    pub source_commit_sha: String,
    #[serde(default)]
    pub generated_at: String,
}

/// Discovered tree snapshot under `prompts_root/trees/<tree_id>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvailableTreeSnapshot {
    pub tree_id: String,
    pub version: Option<KernelVersion>,
    pub version_label: String,
}

/// Result of resolving which prompt tree snapshot applies to a patch review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTreeSnapshot {
    pub tree_id: String,
    pub kernel_version: String,
    pub baseline_version: Option<String>,
    pub version_mismatch_note: Option<String>,
}

impl Default for ResolvedTreeSnapshot {
    fn default() -> Self {
        Self {
            tree_id: "mainline".to_string(),
            kernel_version: "mainline".to_string(),
            baseline_version: None,
            version_mismatch_note: None,
        }
    }
}

/// Reads and parses an optional JSON configuration file. Returns `Ok(None)` when
/// the file does not exist on disk, and returns an `Err` when the file exists
/// but cannot be read or deserialized so corrupted prompt indexes never degrade
/// silently to empty sets.
fn load_optional_json<T>(path: &Path) -> anyhow::Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    use anyhow::Context;
    if !path.is_file() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read prompt JSON {}", path.display()))?;
    let parsed = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse prompt JSON {}", path.display()))?;
    Ok(Some(parsed))
}

/// Validates that a Git revision identifier cannot inject command-line flags or
/// path/range expressions into `git show <rev>:Makefile`.
pub fn is_safe_git_revision(rev: &str) -> bool {
    !rev.is_empty()
        && rev != "unknown"
        && !rev.starts_with('-')
        && rev.len() <= 256
        && !rev.contains("..")
        && !rev.contains(':')
        && rev
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-' | '~' | '^'))
}

/// Discovers all snapshot subdirectories under `<prompts_root>/trees/`.
pub fn discover_tree_snapshots(prompts_root: &Path) -> Vec<AvailableTreeSnapshot> {
    let trees_dir = prompts_root.join("trees");
    let Ok(read_dir) = std::fs::read_dir(&trees_dir) else {
        return Vec::new();
    };
    let mut snapshots = Vec::new();
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(tree_id) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let meta_path = path.join("tree_meta.json");
        let meta: Option<TreeMeta> = match load_optional_json(&meta_path) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("{e}");
                None
            }
        };
        let version_str = meta
            .as_ref()
            .map(|m| m.kernel_version.trim().to_string())
            .filter(|s| !s.is_empty());
        let parsed_version = version_str
            .as_deref()
            .and_then(KernelVersion::parse)
            .or_else(|| KernelVersion::parse(&tree_id));
        let version_label = version_str
            .or_else(|| parsed_version.map(|v| v.to_string()))
            .unwrap_or_else(|| tree_id.clone());
        snapshots.push(AvailableTreeSnapshot {
            tree_id,
            version: parsed_version,
            version_label,
        });
    }
    snapshots.sort_by(|a, b| a.tree_id.cmp(&b.tree_id));
    snapshots
}

/// Reads `<baseline_sha>:Makefile` from `worktree_path` (bounded to 8 KB) and
/// extracts the kernel version.
pub async fn detect_baseline_kernel_version(
    worktree_path: &Path,
    baseline_sha: &str,
) -> Option<KernelVersion> {
    use std::process::Stdio;
    use tokio::io::AsyncReadExt;

    let rev = baseline_sha.trim();
    if !is_safe_git_revision(rev) {
        return None;
    }
    let spec = format!("{rev}:Makefile");
    let mut child = git_cmd::in_dir_async(worktree_path)
        .args(["cat-file", "blob", &spec])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut buf = Vec::with_capacity(1024);
    if let Some(stdout) = child.stdout.take() {
        let _ = stdout.take(8192).read_to_end(&mut buf).await;
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
    if buf.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&buf);
    KernelVersion::parse_makefile(&text)
}

/// Resolves the best matching prompt snapshot for `baseline_version` and optional
/// `branch_hint` across `snapshots`.
pub fn resolve_tree_snapshot_for_version(
    snapshots: &[AvailableTreeSnapshot],
    baseline_version: Option<KernelVersion>,
    branch_hint: Option<&str>,
) -> ResolvedTreeSnapshot {
    if snapshots.is_empty() {
        let ver_str = baseline_version.map(|v| v.to_string());
        return ResolvedTreeSnapshot {
            tree_id: "mainline".to_string(),
            kernel_version: ver_str.clone().unwrap_or_else(|| "mainline".to_string()),
            baseline_version: ver_str,
            version_mismatch_note: None,
        };
    }

    if let Some(hint) = branch_hint.map(str::trim).filter(|s| !s.is_empty()) {
        let head = hint.split('/').next().unwrap_or(hint);
        let tail = hint.rsplit('/').next().unwrap_or(hint);
        if let Some(found) = snapshots
            .iter()
            .find(|s| s.tree_id == hint || s.tree_id == tail || s.tree_id == head)
        {
            return ResolvedTreeSnapshot {
                tree_id: found.tree_id.clone(),
                kernel_version: found.version_label.clone(),
                baseline_version: baseline_version.map(|v| v.to_string()),
                version_mismatch_note: None,
            };
        }
    }

    let Some(target_ver) = baseline_version else {
        let chosen = snapshots
            .iter()
            .find(|s| s.tree_id == "mainline")
            .unwrap_or(&snapshots[0]);
        return ResolvedTreeSnapshot {
            tree_id: chosen.tree_id.clone(),
            kernel_version: chosen.version_label.clone(),
            baseline_version: None,
            version_mismatch_note: None,
        };
    };

    let lts_id = format!("linux-{target_ver}.y");
    if let Some(exact_lts) = snapshots.iter().find(|s| s.tree_id == lts_id) {
        return ResolvedTreeSnapshot {
            tree_id: exact_lts.tree_id.clone(),
            kernel_version: exact_lts.version_label.clone(),
            baseline_version: Some(target_ver.to_string()),
            version_mismatch_note: None,
        };
    }

    if let Some(exact_ver) = snapshots.iter().find(|s| s.version == Some(target_ver)) {
        return ResolvedTreeSnapshot {
            tree_id: exact_ver.tree_id.clone(),
            kernel_version: exact_ver.version_label.clone(),
            baseline_version: Some(target_ver.to_string()),
            version_mismatch_note: None,
        };
    }

    if let Some(mainline) = snapshots.iter().find(|s| s.tree_id == "mainline")
        && mainline.version.is_some_and(|mv| target_ver >= mv)
    {
        return ResolvedTreeSnapshot {
            tree_id: mainline.tree_id.clone(),
            kernel_version: target_ver.to_string(),
            baseline_version: Some(target_ver.to_string()),
            version_mismatch_note: None,
        };
    }

    let nearest_older = snapshots
        .iter()
        .filter(|s| s.version.is_some_and(|v| v <= target_ver))
        .max_by_key(|s| (s.version, s.tree_id == "mainline"));

    if let Some(older) = nearest_older {
        let note = if older.version != Some(target_ver) {
            Some(format!(
                "Target baseline is Linux {}, using nearest older prompt snapshot '{}' (Linux {}). Verify any version-sensitive APIs directly in the checkout.",
                target_ver, older.tree_id, older.version_label
            ))
        } else {
            None
        };
        return ResolvedTreeSnapshot {
            tree_id: older.tree_id.clone(),
            kernel_version: older.version_label.clone(),
            baseline_version: Some(target_ver.to_string()),
            version_mismatch_note: note,
        };
    }

    let fallback = snapshots
        .iter()
        .filter(|s| s.version.is_some())
        .min_by_key(|s| s.version)
        .or_else(|| snapshots.iter().find(|s| s.tree_id == "mainline"))
        .unwrap_or(&snapshots[0]);
    let note = if fallback.version != Some(target_ver) {
        Some(format!(
            "Target baseline is Linux {}, using prompt snapshot '{}' (Linux {}). Do not assume newer kernel APIs exist without checking the source tree.",
            target_ver, fallback.tree_id, fallback.version_label
        ))
    } else {
        None
    };
    ResolvedTreeSnapshot {
        tree_id: fallback.tree_id.clone(),
        kernel_version: fallback.version_label.clone(),
        baseline_version: Some(target_ver.to_string()),
        version_mismatch_note: note,
    }
}

/// Resolves the active tree snapshot for a review against `worktree_path`.
pub async fn resolve_tree_snapshot(
    prompts_root: &Path,
    worktree_path: &Path,
    baseline_sha: &str,
    branch_hint: Option<&str>,
) -> ResolvedTreeSnapshot {
    let root_buf = prompts_root.to_path_buf();
    let snapshots = tokio::task::spawn_blocking(move || discover_tree_snapshots(&root_buf))
        .await
        .unwrap_or_default();
    let baseline_version = detect_baseline_kernel_version(worktree_path, baseline_sha).await;
    resolve_tree_snapshot_for_version(&snapshots, baseline_version, branch_hint)
}

// ---------------------------------------------------------------------------
// Three-Part Index Schemas
// ---------------------------------------------------------------------------

/// Part 1 index (`maintainers/index.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MaintainerNotesIndex {
    #[serde(default)]
    pub entries: Vec<MaintainerNoteIndexEntry>,
}

/// Single entry in `maintainers/index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintainerNoteIndexEntry {
    /// Filename relative to `maintainers/` (e.g. `"networking.md"`).
    pub file: String,
    /// Official `MAINTAINERS` section titles mapped to this note.
    #[serde(default)]
    pub sections: Vec<String>,
    /// Optional file/directory patterns matching directly when touched.
    #[serde(default)]
    pub paths: Vec<String>,
}

/// Part 2 index (`trees/<tree_id>/contexts_index.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeContextsIndex {
    #[serde(default)]
    pub entries: Vec<CodeContextIndexEntry>,
}

/// Single entry in `trees/<tree_id>/contexts_index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeContextIndexEntry {
    /// Path relative to `trees/<tree_id>/contexts/` (e.g. `"mm-vma.md"`).
    pub file: String,
    /// Covered file or directory globs in the kernel source tree.
    #[serde(default)]
    pub covered_paths: Vec<String>,
    /// Optional symbol or keyword triggers.
    #[serde(default)]
    pub triggers: Vec<String>,
}

/// Part 3 index (`trees/<tree_id>/apis_index.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiGuidesIndex {
    #[serde(default)]
    pub entries: Vec<ApiGuideIndexEntry>,
}

/// Symbol triggers for a Part 3 API guide.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiSymbolTriggers {
    #[serde(default)]
    pub exact: Vec<String>,
    #[serde(default)]
    pub prefixes: Vec<String>,
}

/// Single entry in `trees/<tree_id>/apis_index.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGuideIndexEntry {
    /// Path relative to `trees/<tree_id>/apis/` (e.g. `"rcu.md"`).
    pub file: String,
    /// Header or source files defining the API.
    #[serde(default)]
    pub defining_files: Vec<String>,
    /// Function, macro, or type triggers.
    #[serde(default)]
    pub symbols: ApiSymbolTriggers,
}

/// Deterministically resolved three-part prompt paths for a patch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeterministicPromptMatches {
    /// Relative paths under `prompts_root`, e.g. `"maintainers/networking.md"`.
    pub maintainer_notes: Vec<String>,
    /// Relative paths under `prompts_root`, e.g. `"trees/mainline/contexts/mm-vma.md"`.
    pub code_contexts: Vec<String>,
    /// Relative paths under `prompts_root`, e.g. `"trees/mainline/apis/rcu.md"`.
    pub api_guides: Vec<String>,
}

/// Extracts C/Rust/Kconfig identifiers from unified diff hunk lines.
pub fn extract_identifiers_from_diff(diff: &str) -> BTreeSet<String> {
    let mut idents = BTreeSet::new();
    for line in diff.lines() {
        if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("diff --git")
            || line.starts_with("index ")
        {
            continue;
        }
        if !(line.starts_with('+') || line.starts_with('-') || line.starts_with(' ')) {
            continue;
        }
        let body = &line[1..];
        let mut start: Option<usize> = None;
        for (idx, ch) in body.char_indices() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                if start.is_none() && (ch.is_ascii_alphabetic() || ch == '_') {
                    start = Some(idx);
                }
            } else if let Some(s) = start.take() {
                let token = &body[s..idx];
                if token.len() >= 3 {
                    idents.insert(token.to_string());
                }
            }
        }
        if let Some(s) = start {
            let token = &body[s..];
            if token.len() >= 3 {
                idents.insert(token.to_string());
            }
        }
    }
    idents
}

fn max_pattern_depth_for_files(patterns: &[String], touched_files: &[String]) -> Option<usize> {
    let mut best: Option<usize> = None;
    for raw_pat in patterns {
        let compiled = CompiledPattern::compile(raw_pat);
        for file in touched_files {
            if compiled.is_match(file) {
                let d = compiled.depth();
                best = Some(best.map_or(d, |cur| cur.max(d)));
            }
        }
    }
    best
}

/// Matches Part 1 Maintainers Notes against modified files and `MAINTAINERS` sections.
pub fn match_maintainer_notes(
    index: &MaintainerNotesIndex,
    touched_files: &[String],
    matched_sections: &[String],
) -> Vec<String> {
    let mut scored: Vec<(usize, usize, String)> = Vec::new();
    for (entry_idx, entry) in index.entries.iter().enumerate() {
        let rel = format!("maintainers/{}", entry.file.trim_start_matches('/'));
        if !sanitize_prompt_relpath(&rel) {
            continue;
        }
        let section_rank = entry
            .sections
            .iter()
            .filter_map(|sec| {
                matched_sections
                    .iter()
                    .position(|ms| ms.eq_ignore_ascii_case(sec.trim()))
            })
            .min();
        let path_depth = max_pattern_depth_for_files(&entry.paths, touched_files);

        if let Some(pos) = section_rank {
            let priority = 1000usize.saturating_sub(pos);
            scored.push((priority, entry_idx, rel));
        } else if let Some(depth) = path_depth {
            scored.push((depth.saturating_add(1), entry_idx, rel));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let mut out = Vec::new();
    for (_, _, rel) in scored {
        if !out.contains(&rel) {
            out.push(rel);
        }
    }
    out
}

/// Matches Part 2 Code Contexts against modified files and diff identifiers.
pub fn match_code_contexts(
    index: &CodeContextsIndex,
    tree_id: &str,
    touched_files: &[String],
    diff_identifiers: &BTreeSet<String>,
) -> Vec<String> {
    let mut scored: Vec<(usize, usize, String)> = Vec::new();
    for (entry_idx, entry) in index.entries.iter().enumerate() {
        let rel = format!(
            "trees/{}/contexts/{}",
            tree_id,
            entry.file.trim_start_matches('/')
        );
        if !sanitize_prompt_relpath(&rel) {
            continue;
        }
        let path_depth = max_pattern_depth_for_files(&entry.covered_paths, touched_files);
        let trigger_hits = entry
            .triggers
            .iter()
            .filter(|t| {
                let t_trim = t.trim();
                !t_trim.is_empty()
                    && (diff_identifiers.contains(t_trim)
                        || touched_files.iter().any(|f| f.contains(t_trim)))
            })
            .count();
        if let Some(depth) = path_depth {
            let score = 100usize
                .saturating_add(depth.saturating_mul(10))
                .saturating_add(trigger_hits);
            scored.push((score, entry_idx, rel));
        } else if trigger_hits > 0 {
            scored.push((trigger_hits, entry_idx, rel));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let mut out = Vec::new();
    for (_, _, rel) in scored {
        if !out.contains(&rel) {
            out.push(rel);
        }
    }
    out
}

/// Matches Part 3 Kernel API Guides against modified files and diff identifiers.
pub fn match_api_guides(
    index: &ApiGuidesIndex,
    tree_id: &str,
    touched_files: &[String],
    diff_identifiers: &BTreeSet<String>,
) -> Vec<String> {
    let mut scored: Vec<(usize, usize, String)> = Vec::new();
    for (entry_idx, entry) in index.entries.iter().enumerate() {
        let rel = format!(
            "trees/{}/apis/{}",
            tree_id,
            entry.file.trim_start_matches('/')
        );
        if !sanitize_prompt_relpath(&rel) {
            continue;
        }
        let def_depth = max_pattern_depth_for_files(&entry.defining_files, touched_files);
        let exact_hits = entry
            .symbols
            .exact
            .iter()
            .filter(|sym| diff_identifiers.contains(sym.trim()))
            .count();
        let prefix_hits = entry
            .symbols
            .prefixes
            .iter()
            .filter(|pref| {
                let p = pref.trim();
                !p.is_empty() && diff_identifiers.iter().any(|id| id.starts_with(p))
            })
            .count();

        let base_score =
            def_depth.map_or(0usize, |d| 100usize.saturating_add(d.saturating_mul(10)));
        let score = base_score
            .saturating_add(exact_hits.saturating_mul(10))
            .saturating_add(prefix_hits.saturating_mul(3));
        if score > 0 {
            scored.push((score, entry_idx, rel));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));

    let mut out = Vec::new();
    for (_, _, rel) in scored {
        if !out.contains(&rel) {
            out.push(rel);
        }
    }
    out
}

/// Runs deterministic matching for Parts 1, 2, and 3 on `diff` using the index
/// files under `prompts_root` and optional `MAINTAINERS` file in `worktree_path`.
pub fn resolve_deterministic_prompts(
    prompts_root: &Path,
    worktree_path: &Path,
    tree_id: &str,
    diff: &str,
) -> anyhow::Result<DeterministicPromptMatches> {
    let touched_files = extract_files_from_diff(diff);
    let diff_identifiers = extract_identifiers_from_diff(diff);

    let maintainers_path = worktree_path.join("MAINTAINERS");
    let matched_sections = if let Ok(meta) = std::fs::symlink_metadata(&maintainers_path)
        && !meta.file_type().is_symlink()
        && meta.is_file()
        && meta.len() <= 4 * 1024 * 1024
    {
        match MaintainersIndex::from_file(&maintainers_path) {
            Ok(idx) => idx.match_files(&touched_files),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse MAINTAINERS at {}: {}",
                    maintainers_path.display(),
                    e
                );
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let maint_index: MaintainerNotesIndex =
        load_optional_json(&prompts_root.join("maintainers/index.json"))?.unwrap_or_default();
    let contexts_index: CodeContextsIndex = load_optional_json(
        &prompts_root
            .join("trees")
            .join(tree_id)
            .join("contexts_index.json"),
    )?
    .unwrap_or_default();
    let apis_index: ApiGuidesIndex = load_optional_json(
        &prompts_root
            .join("trees")
            .join(tree_id)
            .join("apis_index.json"),
    )?
    .unwrap_or_default();

    Ok(DeterministicPromptMatches {
        maintainer_notes: match_maintainer_notes(&maint_index, &touched_files, &matched_sections),
        code_contexts: match_code_contexts(
            &contexts_index,
            tree_id,
            &touched_files,
            &diff_identifiers,
        ),
        api_guides: match_api_guides(&apis_index, tree_id, &touched_files, &diff_identifiers),
    })
}

/// Partitions `candidates` into `(inlined, overflow)` using precomputed in-memory
/// byte sizes `guide_sizes` so callers on async threads can enforce budgets
/// without blocking filesystem metadata syscalls.
///
/// A candidate missing from `guide_sizes` has an unknown size, so it always
/// goes to `overflow`: inlining it could exceed `max_bytes` by an unbounded
/// amount, while an overflowed guide stays readable on demand.
pub fn apply_part_budget_from_sizes(
    guide_sizes: &std::collections::HashMap<String, usize>,
    candidates: &[String],
    max_bytes: usize,
) -> (Vec<String>, Vec<String>) {
    let mut inlined = Vec::new();
    let mut overflow = Vec::new();
    let mut used = 0usize;

    for rel in candidates {
        if !sanitize_prompt_relpath(rel) {
            continue;
        }
        let Some(len) = guide_sizes.get(rel.as_str()).copied() else {
            overflow.push(rel.clone());
            continue;
        };
        if used.saturating_add(len) <= max_bytes
            || (inlined.is_empty() && len <= max_bytes.saturating_mul(2))
        {
            used = used.saturating_add(len);
            inlined.push(rel.clone());
        } else {
            overflow.push(rel.clone());
        }
    }

    (inlined, overflow)
}

/// Partitions `candidates` into `(inlined, overflow)` according to `max_bytes`.
pub fn apply_part_budget(
    prompts_root: &Path,
    candidates: &[String],
    max_bytes: usize,
) -> (Vec<String>, Vec<String>) {
    let mut inlined = Vec::new();
    let mut overflow = Vec::new();
    let mut used = 0usize;

    for rel in candidates {
        if !sanitize_prompt_relpath(rel) {
            continue;
        }
        let full = prompts_root.join(rel);
        let Ok(meta) = std::fs::symlink_metadata(&full) else {
            continue;
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            continue;
        }
        let len = usize::try_from(meta.len()).unwrap_or(usize::MAX);
        if used.saturating_add(len) <= max_bytes
            || (inlined.is_empty() && len <= max_bytes.saturating_mul(2))
        {
            used = used.saturating_add(len);
            inlined.push(rel.clone());
        } else {
            overflow.push(rel.clone());
        }
    }

    (inlined, overflow)
}

/// Strips purely stylistic or patch-routing sections (e.g. `## Coding Style & Conventions`,
/// `## Welcome vs. Unwelcome Changes`) from a Part 1 Maintainers Note when
/// rendering for low-level technical analysis stages (`execution-flow`,
/// `resources`, `locking`, `security`, `hardware`), keeping only bug/safety-relevant
/// sections.
pub fn filter_maintainer_note_for_technical_stage(content: &str) -> String {
    let mut out = String::new();
    let mut skip_section = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            let lower = heading.to_ascii_lowercase();
            skip_section = lower.contains("coding style")
                || lower.contains("formatting")
                || lower.contains("welcome vs")
                || lower.contains("patch submission")
                || lower.contains("tree routing");
        }
        if !skip_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_kernel_version_parse_and_makefile() {
        assert_eq!(
            KernelVersion::parse("6.14"),
            Some(KernelVersion {
                major: 6,
                minor: 14
            })
        );
        assert_eq!(
            KernelVersion::parse("linux-5.2.y"),
            Some(KernelVersion { major: 5, minor: 2 })
        );
        assert_eq!(
            KernelVersion::parse("6.12-rc3"),
            Some(KernelVersion {
                major: 6,
                minor: 12
            })
        );

        let makefile = "\
# SPDX-License-Identifier: GPL-2.0
VERSION = 5
PATCHLEVEL = 10
SUBLEVEL = 214
EXTRAVERSION =
NAME = Dare mighty things
";
        assert_eq!(
            KernelVersion::parse_makefile(makefile),
            Some(KernelVersion {
                major: 5,
                minor: 10
            })
        );
    }

    #[test]
    fn test_resolve_tree_snapshot_for_version_exact_and_ancestor() {
        let snapshots = vec![
            AvailableTreeSnapshot {
                tree_id: "linux-5.2.y".to_string(),
                version: Some(KernelVersion { major: 5, minor: 2 }),
                version_label: "5.2".to_string(),
            },
            AvailableTreeSnapshot {
                tree_id: "linux-6.6.y".to_string(),
                version: Some(KernelVersion { major: 6, minor: 6 }),
                version_label: "6.6".to_string(),
            },
            AvailableTreeSnapshot {
                tree_id: "mainline".to_string(),
                version: Some(KernelVersion {
                    major: 6,
                    minor: 14,
                }),
                version_label: "6.14".to_string(),
            },
            AvailableTreeSnapshot {
                tree_id: "net-next".to_string(),
                version: Some(KernelVersion {
                    major: 6,
                    minor: 14,
                }),
                version_label: "6.14".to_string(),
            },
        ];

        // Exact LTS match
        let res_52 = resolve_tree_snapshot_for_version(
            &snapshots,
            Some(KernelVersion { major: 5, minor: 2 }),
            None,
        );
        assert_eq!(res_52.tree_id, "linux-5.2.y");
        assert!(res_52.version_mismatch_note.is_none());

        // Nearest-older-ancestor fallback (5.10 -> linux-5.2.y, NOT mainline)
        let res_510 = resolve_tree_snapshot_for_version(
            &snapshots,
            Some(KernelVersion {
                major: 5,
                minor: 10,
            }),
            None,
        );
        assert_eq!(res_510.tree_id, "linux-5.2.y");
        assert!(
            res_510
                .version_mismatch_note
                .as_deref()
                .is_some_and(|n| n.contains("5.10") && n.contains("linux-5.2.y"))
        );

        // Explicit branch hint wins
        let res_net_next = resolve_tree_snapshot_for_version(
            &snapshots,
            Some(KernelVersion {
                major: 6,
                minor: 14,
            }),
            Some("net-next/main"),
        );
        assert_eq!(res_net_next.tree_id, "net-next");

        // Newer than mainline falls back to mainline cleanly
        let res_615 = resolve_tree_snapshot_for_version(
            &snapshots,
            Some(KernelVersion {
                major: 6,
                minor: 15,
            }),
            None,
        );
        assert_eq!(res_615.tree_id, "mainline");
        assert!(res_615.version_mismatch_note.is_none());
    }

    #[test]
    fn test_three_part_deterministic_matching() {
        let maint_idx = MaintainerNotesIndex {
            entries: vec![
                MaintainerNoteIndexEntry {
                    file: "networking.md".to_string(),
                    sections: vec!["NETWORKING [GENERAL]".to_string()],
                    paths: vec!["net/".to_string()],
                },
                MaintainerNoteIndexEntry {
                    file: "bpf.md".to_string(),
                    sections: vec!["BPF [GENERAL]".to_string()],
                    paths: vec!["kernel/bpf/".to_string()],
                },
            ],
        };
        let ctx_idx = CodeContextsIndex {
            entries: vec![
                CodeContextIndexEntry {
                    file: "networking-core.md".to_string(),
                    covered_paths: vec![
                        "net/core/".to_string(),
                        "include/linux/skbuff.h".to_string(),
                    ],
                    triggers: vec!["sk_buff".to_string()],
                },
                CodeContextIndexEntry {
                    file: "mm-vma.md".to_string(),
                    covered_paths: vec!["mm/vma.c".to_string()],
                    triggers: vec![],
                },
            ],
        };
        let api_idx = ApiGuidesIndex {
            entries: vec![
                ApiGuideIndexEntry {
                    file: "rcu.md".to_string(),
                    defining_files: vec!["include/linux/rcupdate.h".to_string()],
                    symbols: ApiSymbolTriggers {
                        exact: vec!["synchronize_rcu".to_string()],
                        prefixes: vec!["rcu_".to_string(), "kfree_rcu".to_string()],
                    },
                },
                ApiGuideIndexEntry {
                    file: "timers.md".to_string(),
                    defining_files: vec!["include/linux/timer.h".to_string()],
                    symbols: ApiSymbolTriggers {
                        exact: vec!["timer_delete_sync".to_string()],
                        prefixes: vec![],
                    },
                },
            ],
        };

        let diff = "\
diff --git a/net/core/skbuff.c b/net/core/skbuff.c
index 1111111..2222222 100644
--- a/net/core/skbuff.c
+++ b/net/core/skbuff.c
@@ -10,4 +10,6 @@ void foo(struct sk_buff *skb)
 {
+    rcu_read_lock();
+    rcu_read_unlock();
 }
";
        let touched = extract_files_from_diff(diff);
        let idents = extract_identifiers_from_diff(diff);
        let sections = vec!["NETWORKING [GENERAL]".to_string()];

        let p1 = match_maintainer_notes(&maint_idx, &touched, &sections);
        assert_eq!(p1, vec!["maintainers/networking.md"]);

        let p2 = match_code_contexts(&ctx_idx, "mainline", &touched, &idents);
        assert_eq!(p2, vec!["trees/mainline/contexts/networking-core.md"]);

        let p3 = match_api_guides(&api_idx, "mainline", &touched, &idents);
        assert_eq!(p3, vec!["trees/mainline/apis/rcu.md"]);
    }

    #[test]
    fn test_apply_part_budget_and_technical_stage_filter() {
        let dir = tempdir().unwrap();
        let maint_dir = dir.path().join("maintainers");
        std::fs::create_dir_all(&maint_dir).unwrap();
        std::fs::write(maint_dir.join("a.md"), "a".repeat(100)).unwrap();
        std::fs::write(maint_dir.join("b.md"), "b".repeat(100)).unwrap();

        let candidates = vec![
            "maintainers/a.md".to_string(),
            "maintainers/b.md".to_string(),
        ];
        let (inlined, overflow) = apply_part_budget(dir.path(), &candidates, 150);
        assert_eq!(inlined, vec!["maintainers/a.md"]);
        assert_eq!(overflow, vec!["maintainers/b.md"]);

        let note = "\
# Networking Maintainers Notes

## Evolution & Architectural Direction
- Use netlink.

## Coding Style & Conventions
- Use Reverse Christmas Tree ordering.

## Error Handling & Safety Rules
- Never call BUG_ON() in packet paths.
";
        let filtered = filter_maintainer_note_for_technical_stage(note);
        assert!(filtered.contains("Use netlink."));
        assert!(filtered.contains("Never call BUG_ON() in packet paths."));
        assert!(!filtered.contains("Reverse Christmas Tree"));
    }

    #[test]
    fn test_apply_part_budget_from_sizes_overflows_unknown_guides() {
        let sizes = std::collections::HashMap::from([
            ("maintainers/a.md".to_string(), 100usize),
            ("maintainers/b.md".to_string(), 100usize),
        ]);
        let candidates = vec![
            "maintainers/unknown.md".to_string(),
            "maintainers/a.md".to_string(),
            "maintainers/b.md".to_string(),
        ];
        let (inlined, overflow) = apply_part_budget_from_sizes(&sizes, &candidates, 150);
        assert_eq!(inlined, vec!["maintainers/a.md"]);
        assert_eq!(overflow, vec!["maintainers/unknown.md", "maintainers/b.md"]);
    }

    #[tokio::test]
    async fn test_is_safe_git_revision_and_malformed_json_fallback() {
        assert!(is_safe_git_revision("HEAD"));
        assert!(is_safe_git_revision("origin/master"));
        assert!(is_safe_git_revision("a1b2c3d4e5f6"));
        assert!(!is_safe_git_revision(""));
        assert!(!is_safe_git_revision("unknown"));
        assert!(!is_safe_git_revision("--output=/tmp/evil"));
        assert!(!is_safe_git_revision("-n"));
        assert!(!is_safe_git_revision("HEAD..origin/main"));
        assert!(!is_safe_git_revision("HEAD:Makefile"));

        let dir = tempdir().unwrap();
        assert_eq!(
            detect_baseline_kernel_version(dir.path(), "--output=/tmp/evil").await,
            None
        );

        let bad_json = dir.path().join("bad.json");
        std::fs::write(&bad_json, "{not valid json").unwrap();
        let parsed: anyhow::Result<Option<MaintainerNotesIndex>> = load_optional_json(&bad_json);
        assert!(parsed.is_err());

        #[cfg(unix)]
        {
            let wt = tempdir().unwrap();
            let target = wt.path().join("real_maintainers");
            std::fs::write(&target, "SUBSYSTEM\nM:\tTest <t@example.com>\nF:\t*\n").unwrap();
            std::os::unix::fs::symlink(&target, wt.path().join("MAINTAINERS")).unwrap();
            let det = resolve_deterministic_prompts(
                dir.path(),
                wt.path(),
                "mainline",
                "diff --git a/foo.c b/foo.c\n",
            )
            .unwrap();
            assert!(det.maintainer_notes.is_empty());
        }
    }
}
