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

//! Security and normalization helpers shared across review workflows.

/// Verifies that a guide name selected by an LLM (e.g. during pre-screening) is
/// a plain filename without path separators or parent directory components.
///
/// Selected guide names are joined against the prompt bundle directory and
/// inlined into stage system prompts. Since patches from untrusted sources can
/// influence LLM selection, any name containing path traversal characters is
/// rejected.
pub fn sanitize_guide_name(name: &str) -> bool {
    let plain =
        !name.is_empty() && !name.contains('/') && !name.contains('\\') && !name.contains("..");
    if !plain {
        tracing::warn!("Ignoring prescreen guide with a path in its name: {name}");
    }
    plain
}

/// Verifies that a relative prompt path (from `def.guides`, `selected_guides`,
/// or `read_prompt`) is a safe `.md` path within the prompt bundle without
/// parent-directory traversal or absolute components.
pub fn sanitize_prompt_relpath(path: &str) -> bool {
    let trimmed = path.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || !trimmed.ends_with(".md")
    {
        return false;
    }
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return false;
    }
    match parts.as_slice() {
        [_file] => true,
        [dir, _file] => matches!(
            *dir,
            "subsystem" | "patterns" | "framing" | "maintainers" | "contexts" | "apis"
        ),
        [dir, rest @ ..] if matches!(*dir, "contexts" | "apis") => !rest.is_empty(),
        ["trees", _tree_id, "index.md"] => true,
        ["trees", _tree_id, kind, rest @ ..] if matches!(*kind, "contexts" | "apis") => {
            !rest.is_empty()
        }
        _ => false,
    }
}

/// Validates and normalizes a tree-scoped prompt relative path returned by an
/// LLM (e.g. `additional_contexts` or `additional_apis` during `pre-screen`)
/// into a canonical path under `trees/<tree_id>/<kind>/...`.
///
/// Accepts either a bare path under `<kind>/` (e.g. `"mm-vma.md"`,
/// `"net/core/skbuff.md"`), a `<kind>/`-prefixed path (e.g.
/// `"contexts/mm-vma.md"`), or a full `"trees/<tree_id>/<kind>/..."` path,
/// while rejecting any traversal or mismatched category.
pub fn normalize_tree_prompt_relpath(
    raw: &str,
    tree_id: &str,
    expected_kind: &str,
) -> Option<String> {
    if !matches!(expected_kind, "contexts" | "apis") {
        return None;
    }
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || !trimmed.ends_with(".md")
    {
        return None;
    }
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts
        .iter()
        .any(|p| p.is_empty() || *p == "." || *p == "..")
    {
        return None;
    }
    let tail: &[&str] = match parts.as_slice() {
        ["trees", tid, kind, rest @ ..] => {
            if *tid != tree_id || *kind != expected_kind || rest.is_empty() {
                return None;
            }
            rest
        }
        [kind, rest @ ..] if matches!(*kind, "contexts" | "apis") => {
            if *kind != expected_kind || rest.is_empty() {
                return None;
            }
            rest
        }
        ["subsystem" | "patterns" | "maintainers" | "framing", ..] => return None,
        rest => rest,
    };
    let normalized = format!("trees/{}/{}/{}", tree_id, expected_kind, tail.join("/"));
    if sanitize_prompt_relpath(&normalized) {
        Some(normalized)
    } else {
        None
    }
}

/// Normalizes a stage name returned by an LLM planner or CLI flag to its
/// canonical lowercase kebab-case identifier without leading `stage-` prefix.
pub fn normalize_stage_name(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase().replace('_', "-");
    if let Some(stripped) = lower.strip_prefix("stage-") {
        stripped.to_string()
    } else {
        lower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_guide_name_accepts_plain_filename() {
        assert!(sanitize_guide_name("locking.md"));
        assert!(sanitize_guide_name("workflow-engine.md"));
        assert!(sanitize_guide_name("rust-async.md"));
    }

    #[test]
    fn test_sanitize_guide_name_rejects_traversal_and_paths() {
        assert!(!sanitize_guide_name(""));
        assert!(!sanitize_guide_name("../etc/passwd"));
        assert!(!sanitize_guide_name("subsystem/locking.md"));
        assert!(!sanitize_guide_name("subsystem\\locking.md"));
        assert!(!sanitize_guide_name(".."));
    }

    #[test]
    fn test_sanitize_prompt_relpath() {
        assert!(sanitize_prompt_relpath("technical-patterns.md"));
        assert!(sanitize_prompt_relpath("callstack.md"));
        assert!(sanitize_prompt_relpath("subsystem/locking.md"));
        assert!(sanitize_prompt_relpath("patterns/rust-async.md"));
        assert!(sanitize_prompt_relpath("maintainers/networking.md"));
        assert!(sanitize_prompt_relpath("contexts/mm-vma.md"));
        assert!(sanitize_prompt_relpath("contexts/net/core/skbuff.md"));
        assert!(sanitize_prompt_relpath("apis/rcu.md"));
        assert!(sanitize_prompt_relpath("trees/mainline/index.md"));
        assert!(sanitize_prompt_relpath("trees/mainline/contexts/mm-vma.md"));
        assert!(sanitize_prompt_relpath(
            "trees/linux-5.2.y/contexts/net/core/skbuff.md"
        ));
        assert!(sanitize_prompt_relpath("trees/mainline/apis/rcu.md"));
        assert!(!sanitize_prompt_relpath(""));
        assert!(!sanitize_prompt_relpath("../secret.md"));
        assert!(!sanitize_prompt_relpath("/etc/passwd.md"));
        assert!(!sanitize_prompt_relpath("other/file.md"));
        assert!(!sanitize_prompt_relpath("subsystem/nested/file.md"));
        assert!(!sanitize_prompt_relpath("maintainers/nested/file.md"));
        assert!(!sanitize_prompt_relpath("trees/mainline/other/file.md"));
        assert!(!sanitize_prompt_relpath("locking.txt"));
    }

    #[test]
    fn test_normalize_tree_prompt_relpath() {
        assert_eq!(
            normalize_tree_prompt_relpath("mm-vma.md", "mainline", "contexts"),
            Some("trees/mainline/contexts/mm-vma.md".to_string())
        );
        assert_eq!(
            normalize_tree_prompt_relpath("net/core/skbuff.md", "mainline", "contexts"),
            Some("trees/mainline/contexts/net/core/skbuff.md".to_string())
        );
        assert_eq!(
            normalize_tree_prompt_relpath("contexts/mm-vma.md", "linux-6.6.y", "contexts"),
            Some("trees/linux-6.6.y/contexts/mm-vma.md".to_string())
        );
        assert_eq!(
            normalize_tree_prompt_relpath("trees/mainline/apis/rcu.md", "mainline", "apis"),
            Some("trees/mainline/apis/rcu.md".to_string())
        );
        assert_eq!(
            normalize_tree_prompt_relpath("apis/rcu.md", "mainline", "contexts"),
            None
        );
        assert_eq!(
            normalize_tree_prompt_relpath("trees/linux-5.2.y/apis/rcu.md", "mainline", "apis"),
            None
        );
        assert_eq!(
            normalize_tree_prompt_relpath("../secret.md", "mainline", "contexts"),
            None
        );
    }

    #[test]
    fn test_normalize_stage_name() {
        assert_eq!(
            normalize_stage_name("Stage_Execution_Flow"),
            "execution-flow"
        );
        assert_eq!(normalize_stage_name("stage-llm-pipeline"), "llm-pipeline");
        assert_eq!(normalize_stage_name("  CONCURRENCY "), "concurrency");
    }
}
