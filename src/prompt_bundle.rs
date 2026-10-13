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

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::project::ProjectId;

include!(concat!(env!("OUT_DIR"), "/prompts_generated.rs"));

const COMPLETE_MARKER: &str = ".sashiko-prompts-complete";

/// The installed prompt directory for a project.
///
/// A missing directory is an error rather than a path returned anyway. Every
/// `@include` resolves against this root and silently yields nothing when the
/// file is absent, which is deliberate for an optional guide but would turn a
/// project with no prompts at all into a review that runs with an empty system
/// prompt and reports nothing.
pub fn project_prompts_path(project: ProjectId) -> Result<PathBuf> {
    project_prompts_path_with_overlay(project, None)
}

/// Resolves the prompt directory for `project`, preferring an optional writable
/// Git-backed `overlay_path` when it exists on disk and falling back to the
/// compiled-in prompt bundle.
pub fn project_prompts_path_with_overlay(
    project: ProjectId,
    overlay_path: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(overlay) = overlay_path {
        let project_subdir = overlay.join(project.prompt_dir());
        if project_subdir.is_dir() {
            return Ok(project_subdir);
        }
        if overlay.is_dir()
            && overlay
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == project.prompt_dir())
        {
            return Ok(overlay.to_path_buf());
        }
    }

    let root = install_prompt_bundle(false)?;
    let path = root.join(project.prompt_dir());
    if !path.is_dir() {
        anyhow::bail!(
            "no prompts for project {project}: expected a {} directory in the prompt bundle at {}",
            project.prompt_dir(),
            path.display()
        );
    }
    Ok(path)
}

/// Returns the compiled-in content of `kernel/severity.md`.
pub fn kernel_severity_guide() -> &'static str {
    PROMPT_BUNDLE_FILES
        .iter()
        .find(|(path, _)| *path == "kernel/severity.md")
        .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
        .expect("kernel/severity.md must exist in prompt bundle")
}

/// Returns the compiled-in content of `sashiko/severity.md`.
pub fn sashiko_severity_guide() -> &'static str {
    PROMPT_BUNDLE_FILES
        .iter()
        .find(|(path, _)| *path == "sashiko/severity.md")
        .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
        .expect("sashiko/severity.md must exist in prompt bundle")
}

/// Returns the compiled-in severity guide for `project`.
pub fn severity_guide(project: ProjectId) -> &'static str {
    match project {
        ProjectId::Linux => kernel_severity_guide(),
        ProjectId::Sashiko => sashiko_severity_guide(),
    }
}

pub fn install_prompt_bundle(force: bool) -> Result<PathBuf> {
    let root = prompt_bundle_root()?;
    let marker = root.join(COMPLETE_MARKER);

    if !force && marker.exists() {
        return Ok(root);
    }

    if force && root.exists() {
        std::fs::remove_dir_all(&root)
            .with_context(|| format!("failed to remove {}", root.display()))?;
    }

    for (relative, content) in PROMPT_BUNDLE_FILES {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(&path, content)
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    std::fs::write(&marker, PROMPT_BUNDLE_REVISION)
        .with_context(|| format!("failed to write {}", marker.display()))?;

    Ok(root)
}

pub fn prompt_bundle_root() -> Result<PathBuf> {
    Ok(crate::utils::data_home()?
        .join("sashiko/prompts")
        .join(PROMPT_BUNDLE_REVISION))
}

/// When `base_dir` points directly into a source checkout's `third_party/prompts/<proj>`
/// or `prompts/<proj>` directory rather than the merged installed bundle, returns the
/// sibling directory (`prompts/<proj>` or `third_party/prompts/<proj>`) via pure
/// lexical path inspection without performing blocking filesystem syscalls.
pub fn companion_prompt_dir(base_dir: &Path) -> Option<PathBuf> {
    let proj_name = base_dir.file_name()?;
    let parent = base_dir.parent()?;
    if parent.file_name().is_some_and(|n| n == "prompts") {
        let grandparent = parent.parent()?;
        if grandparent.file_name().is_some_and(|n| n == "third_party") {
            let repo_root = grandparent.parent()?;
            return Some(repo_root.join("prompts").join(proj_name));
        }
        return Some(grandparent.join("third_party/prompts").join(proj_name));
    }
    None
}

/// Resolves `rel` under `base_dir`, falling back to `companion_prompt_dir(base_dir)`
/// when `base_dir` points directly to a split source-tree prompt directory.
pub fn resolve_in_prompt_dir(base_dir: &Path, rel: impl AsRef<Path>) -> PathBuf {
    let rel = rel.as_ref();
    let primary = base_dir.join(rel);
    if primary.exists() {
        return primary;
    }
    let companion = companion_prompt_dir(base_dir);
    if let Some(ref comp) = companion {
        let secondary = comp.join(rel);
        if secondary.exists() {
            return secondary;
        }
    }
    if let Some(rel_str) = rel.to_str()
        && rel_str.starts_with("trees/")
        && (rel_str.contains("/apis/") || rel_str.contains("/contexts/"))
        && let Some(fname) = rel.file_name()
    {
        let legacy_rel = Path::new("subsystem").join(fname);
        let legacy_primary = base_dir.join(&legacy_rel);
        if legacy_primary.exists() {
            return legacy_primary;
        }
        if let Some(ref comp) = companion {
            let legacy_secondary = comp.join(&legacy_rel);
            if legacy_secondary.exists() {
                return legacy_secondary;
            }
        }
    }
    primary
}

/// Reports whether `path` exists without blocking a Tokio worker thread.
///
/// A missing path or a missing parent directory is a normal "absent"
/// answer. Any other failure (permission denied, descriptor exhaustion,
/// I/O error) is returned so callers never mistake an unreadable guide
/// for an absent one and silently drop it from a prompt.
pub async fn path_exists_async(path: &Path) -> std::io::Result<bool> {
    match tokio::fs::try_exists(path).await {
        Ok(exists) => Ok(exists),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(false)
        }
        Err(e) => Err(std::io::Error::new(
            e.kind(),
            format!("failed to check prompt path {}: {e}", path.display()),
        )),
    }
}

/// Async counterpart of [`resolve_in_prompt_dir`] using `tokio::fs::try_exists`
/// so prompt rendering never performs blocking filesystem metadata checks on
/// Tokio worker threads. Existence checks that fail for reasons other than
/// absence are propagated as errors.
pub async fn resolve_in_prompt_dir_async(
    base_dir: &Path,
    rel: impl AsRef<Path>,
) -> std::io::Result<PathBuf> {
    let rel = rel.as_ref();
    let primary = base_dir.join(rel);
    if path_exists_async(&primary).await? {
        return Ok(primary);
    }
    let companion = companion_prompt_dir(base_dir);
    if let Some(ref comp) = companion {
        let secondary = comp.join(rel);
        if path_exists_async(&secondary).await? {
            return Ok(secondary);
        }
    }
    if let Some(rel_str) = rel.to_str()
        && rel_str.starts_with("trees/")
        && (rel_str.contains("/apis/") || rel_str.contains("/contexts/"))
        && let Some(fname) = rel.file_name()
    {
        let legacy_rel = Path::new("subsystem").join(fname);
        let legacy_primary = base_dir.join(&legacy_rel);
        if path_exists_async(&legacy_primary).await? {
            return Ok(legacy_primary);
        }
        if let Some(ref comp) = companion {
            let legacy_secondary = comp.join(&legacy_rel);
            if path_exists_async(&legacy_secondary).await? {
                return Ok(legacy_secondary);
            }
        }
    }
    Ok(primary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_bundle_contains_kernel_review_core() {
        assert!(
            PROMPT_BUNDLE_FILES
                .iter()
                .any(|(path, _)| *path == "kernel/review-core.md")
        );
    }

    #[test]
    fn test_kernel_severity_guide_not_empty() {
        let guide = kernel_severity_guide();
        assert!(guide.contains("# Severity Levels"));
        assert!(guide.contains("## Critical"));
        assert!(guide.contains("## High"));
    }

    #[test]
    fn test_sashiko_severity_guide_not_empty() {
        let guide = sashiko_severity_guide();
        assert!(guide.contains("# Severity Levels"));
        assert!(guide.contains("Sashiko is not an operating system kernel"));
        assert_eq!(severity_guide(ProjectId::Linux), kernel_severity_guide());
        assert_eq!(severity_guide(ProjectId::Sashiko), sashiko_severity_guide());
    }

    #[test]
    fn test_prompt_bundle_root_uses_xdg_data_home() {
        let temp = tempfile::tempdir().unwrap();
        let old_xdg = std::env::var_os("XDG_DATA_HOME");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", temp.path());
        }

        assert_eq!(
            prompt_bundle_root().unwrap(),
            temp.path()
                .join("sashiko/prompts")
                .join(PROMPT_BUNDLE_REVISION)
        );

        unsafe {
            if let Some(value) = old_xdg {
                std::env::set_var("XDG_DATA_HOME", value);
            } else {
                std::env::remove_var("XDG_DATA_HOME");
            }
        }
    }

    #[test]
    fn test_sashiko_prompt_bundle_integrity() {
        let required_framing = [
            "sashiko/review-core.md",
            "sashiko/severity.md",
            "sashiko/false-positive-guide.md",
            "sashiko/prompt-injection.md",
            "sashiko/github-summary-template.md",
            "sashiko/subsystem/subsystem.md",
        ];
        for path in required_framing {
            assert!(
                PROMPT_BUNDLE_FILES.iter().any(|(p, _)| *p == path),
                "missing framing file in bundle: {path}"
            );
        }

        let subsystem_index = PROMPT_BUNDLE_FILES
            .iter()
            .find(|(p, _)| *p == "sashiko/subsystem/subsystem.md")
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .expect("sashiko/subsystem/subsystem.md must be valid UTF-8");

        let mut referenced_guides = Vec::new();
        for line in subsystem_index.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('|') && trimmed.ends_with('|') {
                let cols: Vec<&str> = trimmed.split('|').map(str::trim).collect();
                // Markdown table row: ["", "Component/Pattern", "Triggers", "File", ""]
                if cols.len() >= 4 {
                    let file_col = cols[cols.len() - 2];
                    if file_col.ends_with(".md") && !file_col.contains(' ') {
                        referenced_guides.push(file_col);
                    }
                }
            }
        }

        assert!(
            !referenced_guides.is_empty(),
            "should parse referenced guides from subsystem.md"
        );

        for guide in referenced_guides {
            let in_subsystem = format!("sashiko/subsystem/{guide}");
            let in_patterns = format!("sashiko/patterns/{guide}");
            let found = PROMPT_BUNDLE_FILES
                .iter()
                .any(|(p, _)| *p == in_subsystem || *p == in_patterns);
            assert!(
                found,
                "guide {guide} referenced in sashiko/subsystem/subsystem.md not found in bundle under subsystem/ or patterns/"
            );
        }
    }
    #[test]
    fn test_project_prompts_path_with_overlay() {
        let temp = tempfile::tempdir().unwrap();
        let kernel_sub = temp.path().join("kernel");
        std::fs::create_dir_all(&kernel_sub).unwrap();

        assert_eq!(
            project_prompts_path_with_overlay(ProjectId::Linux, Some(temp.path())).unwrap(),
            kernel_sub
        );
        assert_eq!(
            project_prompts_path_with_overlay(ProjectId::Linux, Some(&kernel_sub)).unwrap(),
            kernel_sub
        );
    }

    #[test]
    fn test_companion_prompt_dir_and_resolve_in_prompt_dir() {
        let temp = tempfile::tempdir().unwrap();
        let vendored_kernel = temp.path().join("third_party/prompts/kernel");
        let root_kernel = temp.path().join("prompts/kernel");
        std::fs::create_dir_all(vendored_kernel.join("subsystem")).unwrap();
        std::fs::create_dir_all(root_kernel.join("maintainers")).unwrap();
        std::fs::write(vendored_kernel.join("review-core.md"), "core").unwrap();
        std::fs::write(vendored_kernel.join("subsystem/locking.md"), "locking").unwrap();
        std::fs::write(root_kernel.join("maintainers/index.json"), "{}").unwrap();

        assert_eq!(
            companion_prompt_dir(&vendored_kernel),
            Some(root_kernel.clone())
        );
        assert_eq!(
            companion_prompt_dir(&root_kernel),
            Some(vendored_kernel.clone())
        );
        assert_eq!(companion_prompt_dir(temp.path()), None);

        assert_eq!(
            resolve_in_prompt_dir(&vendored_kernel, "maintainers/index.json"),
            root_kernel.join("maintainers/index.json")
        );
        assert_eq!(
            resolve_in_prompt_dir(&root_kernel, "review-core.md"),
            vendored_kernel.join("review-core.md")
        );
        assert_eq!(
            resolve_in_prompt_dir(&root_kernel, "trees/mainline/apis/locking.md"),
            vendored_kernel.join("subsystem/locking.md")
        );
    }

    #[test]
    fn test_kernel_prompt_bundle_integrity() {
        let required_files = ["kernel/review-core.md", "kernel/severity.md"];
        for path in required_files {
            assert!(
                PROMPT_BUNDLE_FILES.iter().any(|(p, _)| *p == path),
                "missing kernel prompt file in bundle: {path}"
            );
        }

        let maint_json = PROMPT_BUNDLE_FILES
            .iter()
            .find(|(p, _)| *p == "kernel/maintainers/index.json")
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .expect("kernel/maintainers/index.json must exist in prompt bundle");
        let maint_index: crate::tree_prompts::MaintainerNotesIndex =
            serde_json::from_str(maint_json).expect("valid MaintainerNotesIndex JSON");
        for entry in &maint_index.entries {
            let expected = format!("kernel/maintainers/{}", entry.file);
            assert!(
                PROMPT_BUNDLE_FILES.iter().any(|(p, _)| *p == expected),
                "maintainer note {expected} missing from bundle"
            );
        }

        let contexts_json = PROMPT_BUNDLE_FILES
            .iter()
            .find(|(p, _)| *p == "kernel/trees/mainline/contexts_index.json")
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .expect("kernel/trees/mainline/contexts_index.json must exist in prompt bundle");
        let contexts_index: crate::tree_prompts::CodeContextsIndex =
            serde_json::from_str(contexts_json).expect("valid CodeContextsIndex JSON");
        for entry in &contexts_index.entries {
            let expected = format!("kernel/trees/mainline/contexts/{}", entry.file);
            assert!(
                PROMPT_BUNDLE_FILES.iter().any(|(p, _)| *p == expected),
                "code context {expected} missing from bundle"
            );
        }

        let apis_json = PROMPT_BUNDLE_FILES
            .iter()
            .find(|(p, _)| *p == "kernel/trees/mainline/apis_index.json")
            .and_then(|(_, bytes)| std::str::from_utf8(bytes).ok())
            .expect("kernel/trees/mainline/apis_index.json must exist in prompt bundle");
        let apis_index: crate::tree_prompts::ApiGuidesIndex =
            serde_json::from_str(apis_json).expect("valid ApiGuidesIndex JSON");
        for entry in &apis_index.entries {
            let expected = format!("kernel/trees/mainline/apis/{}", entry.file);
            assert!(
                PROMPT_BUNDLE_FILES.iter().any(|(p, _)| *p == expected),
                "API guide {expected} missing from bundle"
            );
        }
    }
}
