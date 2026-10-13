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

use crate::toolbox::SashikoToolContext;
use crate::toolbox::framework::LlmTool;
use crate::toolbox::utils::validate_path;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct ReadPromptTool;

#[async_trait]
impl LlmTool<SashikoToolContext> for ReadPromptTool {
    fn name(&self) -> &'static str {
        "read_prompt"
    }

    fn description(&self) -> &'static str {
        "Read a specific prompt file from the prompt registry (e.g., 'mm.md', 'locking.md')."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Name of the prompt file (e.g., 'patterns/BPF-001.md')." }
            },
            "required": ["name"]
        })
    }

    async fn call(&self, args: Value, context: &SashikoToolContext) -> Result<Value> {
        let prompts_path = context
            .prompts_path
            .as_ref()
            .ok_or_else(|| anyhow!("read_prompt tool is not available"))?;
        let name = args["name"]
            .as_str()
            .ok_or_else(|| anyhow!("Missing prompt name"))?
            .trim();

        let tree_id = context
            .active_tree_id
            .read()
            .ok()
            .and_then(|g| g.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "mainline".to_string());

        let candidates = prompt_candidates(name, &tree_id);
        let prompts_path_buf = prompts_path.to_path_buf();
        let name_owned = name.to_string();
        let path = tokio::task::spawn_blocking(move || {
            resolve_prompt_candidate(&prompts_path_buf, &candidates, &name_owned)
        })
        .await??;
        let (content, truncated) = read_capped(&path, MAX_READ_PROMPT_BYTES).await?;

        Ok(json!({ "content": content, "truncated": truncated }))
    }
}

/// Upper bound on the bytes `read_prompt` returns. Every shipped guide is far
/// below it; it only stops an oversized or corrupt file from flooding the
/// conversation when the model fetches an overflow guide on demand.
const MAX_READ_PROMPT_BYTES: usize = 128 * 1024;

/// Reads at most `max_bytes` of `path`, cutting a longer file at its last
/// newline within the cap, or at the last UTF-8 character boundary when the
/// capped prefix has no newline, and appending a truncation marker. Returns
/// the content and whether it was truncated.
async fn read_capped(path: &Path, max_bytes: usize) -> Result<(String, bool)> {
    use tokio::io::AsyncReadExt;
    let file = fs::File::open(path).await?;
    let mut buf = Vec::new();
    file.take(max_bytes as u64 + 1)
        .read_to_end(&mut buf)
        .await?;
    let truncated = buf.len() > max_bytes;
    if truncated {
        let end = match buf[..max_bytes].iter().rposition(|b| *b == b'\n') {
            Some(i) => i + 1,
            None => {
                // Step back over UTF-8 continuation bytes (0b10xx_xxxx) so the
                // cut lands before the lead byte of a split character.
                let mut end = max_bytes;
                while end > 0 && (buf[end] & 0xC0) == 0x80 {
                    end -= 1;
                }
                end
            }
        };
        buf.truncate(end);
    }
    let mut content = String::from_utf8_lossy(&buf).into_owned();
    if truncated {
        content.push_str(&format!(
            "\n[read_prompt: output truncated at {max_bytes} bytes]\n"
        ));
    }
    Ok((content, truncated))
}

/// Expands a requested prompt `name` into lookup candidates in priority
/// order. The literal name always comes first; bare names and legacy
/// `subsystem/` names then fall back to the active tree's contexts and APIs,
/// the maintainer notes, and finally the flat legacy directories.
fn prompt_candidates(name: &str, tree_id: &str) -> Vec<String> {
    let mut candidates = vec![name.to_string()];
    if name.starts_with("contexts/") || name.starts_with("apis/") {
        candidates.push(format!("trees/{tree_id}/{name}"));
    } else if let Some(rest) = name.strip_prefix("subsystem/") {
        candidates.push(format!("trees/{tree_id}/contexts/{rest}"));
        candidates.push(format!("trees/{tree_id}/apis/{rest}"));
        candidates.push(format!("maintainers/{rest}"));
    } else if !name.contains('/') {
        candidates.push(format!("trees/{tree_id}/contexts/{name}"));
        candidates.push(format!("trees/{tree_id}/apis/{name}"));
        candidates.push(format!("maintainers/{name}"));
        candidates.push(format!("subsystem/{name}"));
        candidates.push(format!("patterns/{name}"));
    } else if name.starts_with("trees/")
        && (name.contains("/contexts/") || name.contains("/apis/"))
        && let Some(fname) = name.rsplit('/').next()
    {
        // Mirrors the legacy fallback of resolve_in_prompt_dir, so a
        // tree-scoped overflow guide that is backed by a flat legacy file
        // stays readable under the name it was advertised with.
        candidates.push(format!("subsystem/{fname}"));
    }
    candidates
}

/// Returns the first candidate that is a regular file under `prompts_path`
/// or its companion directory, checking the primary directory before the
/// companion for each candidate. Every candidate passes `validate_path`, so
/// a traversal attempt fails instead of escaping the prompt directory.
/// Performs blocking filesystem checks; call it from a blocking thread.
fn resolve_prompt_candidate(
    prompts_path: &Path,
    candidates: &[String],
    name: &str,
) -> Result<PathBuf> {
    let companion = crate::prompt_bundle::companion_prompt_dir(prompts_path);
    for candidate in candidates {
        let p = validate_path(candidate, prompts_path)?;
        if p.is_file() {
            return Ok(p);
        }
        if let Some(comp) = companion.as_ref()
            && let Ok(cp) = validate_path(candidate, comp)
            && cp.is_file()
        {
            return Ok(cp);
        }
    }
    validate_path(name, prompts_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_prompt_candidates_priority_order() {
        assert_eq!(
            prompt_candidates("net.md", "linux-6.6.y"),
            vec![
                "net.md",
                "trees/linux-6.6.y/contexts/net.md",
                "trees/linux-6.6.y/apis/net.md",
                "maintainers/net.md",
                "subsystem/net.md",
                "patterns/net.md",
            ]
        );
        assert_eq!(
            prompt_candidates("subsystem/net.md", "mainline"),
            vec![
                "subsystem/net.md",
                "trees/mainline/contexts/net.md",
                "trees/mainline/apis/net.md",
                "maintainers/net.md",
            ]
        );
        assert_eq!(
            prompt_candidates("apis/rcu.md", "mainline"),
            vec!["apis/rcu.md", "trees/mainline/apis/rcu.md"]
        );
        assert_eq!(
            prompt_candidates("patterns/BPF-001.md", "mainline"),
            vec!["patterns/BPF-001.md"]
        );
    }

    #[test]
    fn test_resolve_prompt_candidate_prefers_tree_and_companion() {
        let repo = tempdir().unwrap();
        let primary = repo.path().join("third_party/prompts/kernel");
        let companion = repo.path().join("prompts/kernel");
        std::fs::create_dir_all(primary.join("subsystem")).unwrap();
        std::fs::create_dir_all(companion.join("trees/mainline/contexts")).unwrap();
        std::fs::write(primary.join("subsystem/net.md"), "legacy").unwrap();
        std::fs::write(companion.join("trees/mainline/contexts/net.md"), "tree").unwrap();

        let candidates = prompt_candidates("net.md", "mainline");
        let resolved = resolve_prompt_candidate(&primary, &candidates, "net.md").unwrap();
        assert_eq!(std::fs::read_to_string(resolved).unwrap(), "tree");

        let candidates = prompt_candidates("subsystem/net.md", "mainline");
        let resolved = resolve_prompt_candidate(&primary, &candidates, "subsystem/net.md").unwrap();
        assert_eq!(std::fs::read_to_string(resolved).unwrap(), "legacy");
    }

    #[test]
    fn test_resolve_prompt_candidate_rejects_traversal() {
        let dir = tempdir().unwrap();
        let candidates = prompt_candidates("../secret.md", "mainline");
        assert!(resolve_prompt_candidate(dir.path(), &candidates, "../secret.md").is_err());
    }

    #[tokio::test]
    async fn test_read_capped_truncates_at_line_boundary() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("big.md");
        std::fs::write(&path, "aaaa\nbbbb\ncccc\n").unwrap();

        let (content, truncated) = read_capped(&path, 12).await.unwrap();
        assert!(truncated);
        assert!(content.starts_with("aaaa\nbbbb\n"));
        assert!(!content.contains("cccc"));
        assert!(content.contains("output truncated at 12 bytes"));

        let (content, truncated) = read_capped(&path, 15).await.unwrap();
        assert!(!truncated);
        assert_eq!(content, "aaaa\nbbbb\ncccc\n");

        // "é" is two bytes (0xC3 0xA9); a cap of 4 splits the second "é".
        let utf8 = dir.path().join("utf8.md");
        std::fs::write(&utf8, "aéé").unwrap();
        let (content, truncated) = read_capped(&utf8, 4).await.unwrap();
        assert!(truncated);
        assert!(content.starts_with("aé\n[read_prompt"));
        assert!(!content.contains('\u{FFFD}'));
    }

    #[test]
    fn test_prompt_candidates_tree_name_falls_back_to_legacy() {
        assert_eq!(
            prompt_candidates("trees/mainline/contexts/net.md", "mainline"),
            vec!["trees/mainline/contexts/net.md", "subsystem/net.md"]
        );
    }
}
