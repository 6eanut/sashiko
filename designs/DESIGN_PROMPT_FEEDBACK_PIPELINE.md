# Design: Continuous Human Feedback & Maintainer Reply Differential Analysis Pipeline

> **Prerequisite Design:** This project builds on the Three-Part Multi-Version
> Kernel Prompt Architecture (`maintainers/*.md`, `trees/<tree_id>/contexts/*.md`,
> and `trees/<tree_id>/apis/*.md`) defined in
> [`designs/DESIGN_KERNEL_PROMPTS_V2.md`](./DESIGN_KERNEL_PROMPTS_V2.md).

## 1. Problem Statement & Motivation

Kernel maintainers, reviewers, and patch authors continuously review patches on
LKML, subsystem mailing lists, and GitHub/GitLab PRs, or provide direct feedback
on Sashiko's findings. Today, this domain knowledge is lost unless a developer
manually edits a prompt file. Two major learning opportunities exist:

1. **Direct Human Feedback on Sashiko Findings:**
   Humans reply to Sashiko's review emails or PR comments, or submit feedback in
   the Web UI/CLI, explaining why a reported finding is a false positive (e.g.,
   due to an unwritten caller invariant or API guarantee) or pointing out a
   subsystem policy rule.
2. **Maintainer Replies to Patches Reviewed by Sashiko (Differential Gap Analysis):**
   Maintainers regularly review patches that Sashiko has also reviewed—even when
   Sashiko stayed silent (0 findings, embargoed, or `dry_run = true`). Comparing
   what human maintainers point out against Sashiko's internal review trace
   (`findings`, `dismissed_concerns`, `review_map`, and `selected_guides`)
   reveals **what types of issues maintainers catch that Sashiko misses** and
   whether Sashiko missed an issue because:
   - a specialist stage spotted it (`C*`/`D*`) and Sashiko **wrongly dismissed**
     it (`false_dismissal`),
   - an existing guide documented the rule but was **not injected**
     (`selection_gap`), or
   - the subjective policy (Part 1), subsystem code context (Part 2), or kernel
     API contract (Part 3) was **absent from the prompt tree** (`unspotted_miss`).

At the same time, Sashiko **cannot blindly trust human claims** on public mailing
lists: patch authors may make false technical claims to excuse a real bug, even
maintainers can misremember subtle helper semantics or misread a diff hunk, and
untrusted participants may attempt prompt injection or try to set subsystem
policy without formal `MAINTAINERS` standing.

---

## 2. Core Epistemic & Security Model

Human feedback and mailing-list review replies arrive from untrusted or
semi-trusted channels. The pipeline enforces four non-negotiable invariants:

1. **Never Trust Claims Blindly:** Every human claim or review critique is
   treated as an unverified hypothesis until proven.
2. **Verify Against Code Whenever Possible:** Any claim about how code or an API
   works (locking, lifecycles, return values, caller invariants, build configs)
   is an **Objective Claim** and MUST be verified against the kernel Git tree
   using tools (`git_read_files`, `git_grep`, `git_show`).
3. **Never Accept Claims Proven Wrong by Code:** If kernel source code refutes a
   claim, the claim is **rejected unconditionally**—even if the author is a
   listed subsystem maintainer (`M:`) or Linus Torvalds. Maintainers are human
   and can misremember whether an internal helper drops a lock or handles `NULL`
   on an obscure error path.
4. **Formal `MAINTAINERS` Standing Governs Subjective Claims:** If and only if a
   claim is genuinely **Subjective / Policy** (cannot be proven or disproven
   from C/Rust source code, and does not ask Sashiko to ignore a verifiable bug),
   acceptance is gated strictly on whether the feedback author holds formal
   authority (`M:` Maintainer, `R:` Reviewer, or `THE REST` Global Maintainer)
   in `MAINTAINERS` for the affected subsystem section.

---

## 3. End-to-End Pipeline Architecture

```mermaid
flowchart TD
    subgraph Ingestion [1. Feedback & Maintainer Reply Sources]
        PatchReply[Maintainer Review Reply on Patch k/N\nNon-patch message in reviewed patchset thread]
        DirectReply[Direct Reply to Sashiko\nLKML / GitHub PR / Web UI / CLI]
    end

    PatchReply --> TargetRes[2a. Target Patch & Thread Resolver\nSkip tag-only replies\nWalk in_reply_to / references_hdr -> patch_id\nInclude sub-thread author rebuttals]
    TargetRes --> DiffComp[2b. Stage 0: Maintainer vs. Sashiko Differential Analyzer\nCompare against reviews.result_description:\nfindings, dismissed_concerns, review_map, selected_guides]

    DiffComp -- "already_caught" --> RecordCaught[Record True Positive Match\nin maintainer_reply_analyses]
    DiffComp -- "false_dismissal\nselection_gap\nunspotted_miss" --> Triage

    DirectReply --> Triage[3. Stage 1: Claim Extractor & Taxonomy Classifier\nClassify into:\n- ObjectiveCodeContext (Part 2)\n- ObjectiveApiContract (Part 3)\n- SubjectiveMaintainerPolicy (Part 1)\n- TriggerIndexUpdate\n- PatchSpecificLogicBug / NotActionable]

    Triage --> AuthCheck[4. Deterministic Authority & Auth Verifier\nCheck sender email in MaintainersIndex\nVerify DKIM/DMARC/Session authenticity]

    AuthCheck --> CodeVerify[5. Stage 2: Adversarial Code Verifier\nToolScope::GitOnly at target_commit_sha, baseline_sha & mainline]

    CodeVerify --> Gate{6. Epistemic Decision Gate}

    Gate -- "Code Refutes Claim\n(VerifiedFalse)" --> RejectFalse[REJECT Unconditionally\nEven if author is M: Maintainer]
    Gate -- "Objective Claim\n+ Code Proves Claim (VerifiedTrue)" --> SynthP23[Stage 3: Synthesize Part 2 / Part 3 / Index Diff\nDetermine applicable tree_ids]
    Gate -- "Objective Claim\n+ Unverifiable in Code" --> RejectUnver[REJECT\nNever take code claims on faith]
    Gate -- "Subjective Claim\n+ No Code Contradiction\n+ Verified M: or R: for Section" --> SynthP1[Stage 3: Synthesize Part 1 Diff\nmaintainers/<slug>.md]
    Gate -- "Subjective Claim\n+ Unlisted / Unverified Sender" --> RejectAuth[REJECT\nInsufficient MAINTAINERS Authority]

    SynthP23 & SynthP1 --> SanitizeGuard[7. Anti-Injection & Boundary Validator\nNeutral 3rd-person rewrite\nNo @include / template tokens\nStrict Part 1 vs Part 2/3 check]

    SanitizeGuard --> ApprovalQueue[8. Staged Approval Queue\nprompt_update_proposals table\nOperator Review in Web UI / CLI]
    ApprovalQueue -- "Operator Approves" --> GitCommit[Commit to Overlay Git Repo\nwith Provenance Trailers]
```

---

## 4. Ingestion Triggers & Target-Patch Resolution

1. **Maintainer Replies to Patches Sashiko Reviewed (`src/main.rs`, `src/ingestor.rs`, `src/db.rs`):**
   - Every mailing-list message in a reviewed thread is stored in `messages`
     even when `metadata.is_patch_or_cover == false`.
   - Crucially, Sashiko can learn from a maintainer's review of Patch $k/N$
     **even when Sashiko never sent an email to the list** (e.g., when Sashiko
     ran with `dry_run = true`, when the series was under embargo, or when
     Sashiko found 0 issues and stayed silent), because Sashiko's full internal
     review record (`reviews`, `findings`, `dismissed_concerns`, `review_map`,
     and `selected_guides`) is already persisted in SQLite.
   - **Deterministic Tag-Only Filter (0 LLM Tokens):** Before queueing an LLM
     evaluation, Sashiko strips quoted lines (`> ...`) and email signatures. If
     the remaining body consists exclusively of trailers or merge notices
     (`Reviewed-by:`, `Acked-by:`, `Tested-by:`, `Signed-off-by:`,
     `Applied to ...`, `Pulled, thanks`), the message is ignored.
   - **Target Patch & Review Resolution (`ResolveReplyTargetPatch`):**
     - Walk `in_reply_to` and `references_hdr` in `messages` to locate the
       nearest ancestor belonging to the patchset:
       1. **Reply to a Specific Patch (`patches.message_id`):** Links directly
          to `patch_id` and its completed `reviews` row (`review_id`). Quoted
          diff context (`> @@ ... @@`, `> +...`, `> -...`) is extracted from the
          reply body to anchor the maintainer's comments to specific files,
          functions, and lines in the patch.
       2. **Reply to the Cover Letter (`patchsets.cover_letter_message_id`):**
          Links to `patchset_id` and resolves candidate patches by matching
          referenced file/symbol names against `patches.diff` (or series-level
          `goal` review).
       3. **Direct Reply to Sashiko (`email_outbox`):** Links directly to the
          `review_id` and `finding_id` that Sashiko sent.
   - **Thread-Settling Window for Author Rebuttals (`reply_settle_seconds`):**
     - On LKML, a reviewer occasionally raises a suspected issue that the patch
       author refutes shortly thereafter with concrete code evidence (e.g.,
       *"`rtnl_lock()` is already held by the caller in `rtnetlink.c`"*).
     - Maintainer reply evaluations wait for a configurable settling window
       (default `3600` seconds) or re-run when a child reply arrives in that
       sub-thread, passing the **entire sub-thread conversation** (maintainer
       comment + author reply + maintainer follow-up) to the analyzer.
2. **Forge Review Replies (`src/forge.rs`):**
   - Webhook events for PR review comments posted by repository maintainers on a
     PR diff or replying to `sashiko-bot` enqueue a differential evaluation task
     with the exact PR file and line range.
3. **Explicit UI / CLI Feedback (`POST /api/prompts/feedback`):**
   - Authenticated maintainers or operators can flag a finding as a false
     positive, report a missed issue on a reviewed patch, or submit a direct
     rule suggestion.

---

## 5. Stage 0: Maintainer Patch-Reply Differential Analysis (`CompareMaintainerReplyStage`)

When a maintainer or reviewer replies to a patch that Sashiko reviewed, Sashiko
loads its stored review artifacts from `reviews.result_description` for that
`patch_id`:
- `findings`: Issues Sashiko reported in its final output.
- `dismissed_concerns`: Concerns Sashiko explicitly investigated and dismissed,
  including each dismissal's `id` (`D1`, `D2`, ...), `locations`, and
  `reasoning`.
- `review_map`: The full trace built by `src/workflows/review_map.rs`, tracking
  every specialist stage's `all_concerns` (`C1`, `C2`, ...),
  `all_dismissed_concerns`, `verification_findings`, `verification_dismissed`,
  `post_verification_findings`, and `post_verification_dismissed`.
- `selected_guides`: The exact Part 1, Part 2, and Part 3 prompt files injected
  during that patch's review.

`CompareMaintainerReplyStage` extracts each distinct review critique from the
maintainer's email and classifies both **how Sashiko handled it**
(`differential_outcome`) and **what kind of issue it is**
(`missed_issue_taxonomy`):

```rust
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MaintainerPointComparison {
    /// Concise summary of the issue pointed out by the maintainer.
    pub maintainer_issue_summary: String,
    /// Verbatim quote (including quoted diff lines) from the maintainer reply.
    pub source_quote: String,
    /// Files and symbols in the patch targeted by the maintainer's comment.
    pub target_locations: Vec<String>,

    /// Differential comparison against Sashiko's review of the same patch:
    /// - "already_caught": Sashiko reported this exact issue in `findings`
    /// - "false_dismissal": A Sashiko stage spotted it (`C*`/`D*`), but Sashiko dismissed it
    /// - "selection_gap": Covered by an existing Part 1/2/3 guide that was NOT in `selected_guides`
    /// - "unspotted_miss": Never raised by any Sashiko stage and not covered by existing guides
    /// - "author_refuted_reviewer": Subsequent reply or code check showed the patch is actually safe
    /// - "non_review_comment": Question, ack, or discussion not pointing out a flaw
    pub differential_outcome: String,

    /// If `already_caught`, the matching finding ID in Sashiko's review.
    pub matched_finding_id: Option<String>,
    /// If `false_dismissal`, the stage (`goal`, `locking`, `verification`, `post-verification`, etc.)
    /// and concern/dismissal ID (`C3`, `D2`) where Sashiko dismissed the issue, plus diagnosis of
    /// why Sashiko's dismissal reasoning was wrong.
    pub dismissed_in_stage: Option<String>,
    pub matched_dismissed_id: Option<String>,
    pub dismissal_flaw_diagnosis: Option<String>,

    /// Taxonomy of what type of issue the maintainer pointed out:
    /// - "subjective_architecture_or_direction" (Part 1: maintainers/*.md)
    /// - "subjective_process_or_tree_routing"   (Part 1: maintainers/*.md)
    /// - "subjective_style_or_convention"       (Part 1: maintainers/*.md)
    /// - "subsystem_state_or_lifecycle_invariant" (Part 2: trees/<tree_id>/contexts/*.md)
    /// - "subsystem_locking_or_execution_context" (Part 2: trees/<tree_id>/contexts/*.md)
    /// - "api_contract_or_ownership_pitfall"    (Part 3: trees/<tree_id>/apis/*.md)
    /// - "hardware_abi_or_spec_constraint"      (Part 2 or Part 3)
    /// - "patch_specific_logic_bug"             (Non-generalizable one-off bug in patch logic)
    /// - "other"                                (Escape hatch)
    pub missed_issue_taxonomy: String,
}
```

### 5.1 Taxonomy of Issues Maintainers Point Out That Sashiko Misses

| Missed-Issue Taxonomy | Typical Examples from Maintainer Replies | Why Sashiko Missed It | Target Improvement Action |
| :--- | :--- | :--- | :--- |
| **`subjective_architecture_or_direction`** | *"Don't add a new `module_param` or private `ioctl`; expose this via `devlink`/`ethtool` netlink"*; *"Put this helper in the core framework, not the driver"*; *"We are deprecating legacy API `X` in favor of `Y`"*. | The C code is memory-safe and bug-free; only the maintainer's architectural roadmap makes it unwelcome. | **Part 1 (`maintainers/<slug>.md`):** Add architectural direction rule (gated on `M:` / `R:` authority). |
| **`subjective_process_or_tree_routing`** | *"This mixes refactoring with a fix; target `net` with a minimal fix + `Fixes:` tag and send the cleanup to `net-next`"*; *"Add a selftest under `tools/testing/selftests/`"*. | Subsystem-specific patch submission and tree-routing policy not derivable from code. | **Part 1 (`maintainers/<slug>.md`):** Add welcome/unwelcome change and tree-routing rule. |
| **`subjective_style_or_convention`** | *"Sort local variables in Reverse Christmas Tree order"*; *"Use `drm_dbg_kms()` instead of `dev_dbg()`"*; *"Don't use `BUG_ON()` here; use `WARN_ON_ONCE()` and recover"*. | Subsystem formatting or error-handling convention. | **Part 1 (`maintainers/<slug>.md`):** Add coding style / error-recovery convention. |
| **`subsystem_state_or_lifecycle_invariant`** | *"At `.ndo_stop` time, workqueue `X` can re-arm itself unless `flag Y` is cleared before `cancel_delayed_work_sync()`"*; *"Callback `A` runs before `B` is initialized on resume"*. | Multi-file state-machine transition or callback ordering invariant in the subsystem. | **Part 2 (`trees/<tree_id>/contexts/<subsys>.md`):** Document lifecycle/state ordering invariant after code verification. |
| **`subsystem_locking_or_execution_context`** | *"This callback runs in softirq / timer / RCU read-side context, so `mutex_lock()` or `GFP_KERNEL` sleeps in atomic context"*; *"Field `foo->bar` is protected by `foo->lock`, not `rtnl_lock`"*. | Execution context of a vtable callback or lock protection of a struct field was unknown to Sashiko. | **Part 2 (`trees/<tree_id>/contexts/<subsys>.md`):** Document callback calling context and lock hierarchy for `<tree_id>`. |
| **`api_contract_or_ownership_pitfall`** | *"`skb_unshare()` frees the old `skb` on failure"*; *"`of_get_next_parent()` drops the input node's refcount"*; *"`snprintf()` returns the would-be length (can exceed buffer size), use `scnprintf()`"*. | Subtle ownership transfer, error return (`ERR_PTR` vs `NULL`), or side effect of a kernel API. | **Part 3 (`trees/<tree_id>/apis/<api>.md`):** Document API contract + add symbol trigger to `apis_index.json`. |
| **`hardware_abi_or_spec_constraint`** | *"Missing PCI posted-write flush (`readl`) before delay"*; *"Struct passed to firmware/UAPI needs `__le32` / explicit padding for 32/64-bit compat"*; *"DMA buffer cannot be on the stack"*. | Hardware bus ordering, DMA rule, or UAPI ABI alignment rule. | **Part 2 (`contexts/`)** for device-specific rules or **Part 3 (`apis/`)** for DMA/PCI/UAPI helpers. |
| **`patch_specific_logic_bug`** | One-off typo, inverted boolean condition, or wrong arithmetic in patch-local logic. | Pure reasoning miss (or `false_dismissal` during `verification`). | Recorded in `maintainer_reply_analyses`; if `false_dismissal`, flags the stage dismissal pattern for core workflow audit. |

### 5.2 Special Handling for `false_dismissal` vs. `selection_gap`
1. **Diagnosing `false_dismissal` (When Sashiko Spotted the Bug and Dismissed It):**
   - Because `review_map` preserves every stage's `all_concerns` and
     `dismissed_concerns` (including `reasoning`), `CompareMaintainerReplyStage`
     checks whether any Sashiko stage raised a concern on the same file/function
     that was subsequently dismissed.
   - Example: Sashiko's `locking` stage raised `C2` (*"Possible sleep in atomic
     context in `foo_timer_cb`"*), and `verification` dismissed it as `D2`
     (*"Assumed `foo_timer_cb` is a workqueue handler that can sleep"*). When
     the maintainer replies pointing out that `foo_timer_cb` is a `timer_list`
     softirq callback, Sashiko identifies `differential_outcome = "false_dismissal"`,
     records `dismissed_in_stage = "verification"` and `matched_dismissed_id = "D2"`,
     and generates a Part 2 context rule explicitly stating that `foo_timer_cb`
     runs in softirq context.
2. **Fixing `selection_gap` (When the Prompt Existed But Was Not Injected):**
   - If the rule already existed in `trees/<tree_id>/contexts/` or
     `trees/<tree_id>/apis/` (checked via keyword/symbol search across the
     prompt bundle), but that file was absent from `selected_guides` for the
     patch, Sashiko synthesizes an index update proposal:
     - Adding the patch's directory/file pattern to `covered_paths` in Part 2,
     - Adding the called helper/macro to `symbols` in `apis_index.json` in
       Part 3, or
     - Clarifying the trigger description in `trees/<tree_id>/index.md` for
       `pre-screen`.

---

## 6. Claim Extraction, Verification, and Synthesis Stages

### 6.1 Stage 1: Claim Extraction & Classification (`ExtractFeedbackClaimsStage`)

Both direct feedback (Mode A) and actionable gaps from `CompareMaintainerReplyStage`
(Mode B: `false_dismissal`, `selection_gap`, `unspotted_miss`) produce structured
`ExtractedClaim` records:

```rust
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ExtractedClaim {
    /// Classification of the claim:
    /// - "objective_code_context": How a specific subsystem/file works (Part 2)
    /// - "objective_api_contract": How a kernel API behaves/expects callers to behave (Part 3)
    /// - "subjective_maintainer_policy": Evolution direction, welcome/unwelcome changes, style (Part 1)
    /// - "trigger_index_update": Existing Part 2/3 guide missed during selection (Selection Gap)
    /// - "not_actionable": Patch-specific one-off remark, greeting, or non-generalizable typo
    pub category: String,
    /// Fine-grained missed-issue taxonomy from Section 5.1.
    pub missed_issue_taxonomy: String,
    /// Differential outcome ("false_dismissal" | "selection_gap" | "unspotted_miss" | "direct_feedback").
    pub differential_outcome: String,
    /// Concise neutral statement of the claimed rule or fact.
    pub claim_summary: String,
    /// Exact quote from the feedback or maintainer reply message.
    pub source_quote: String,
    /// Referenced files, symbols, or APIs if any.
    pub referenced_symbols_or_files: Vec<String>,
    /// Whether this feedback is refuting a Sashiko finding or stating a missed rule.
    pub feedback_intent: String, // "refute_finding" | "missed_concern" | "general_guidance" | "other"
}
```

If all extracted items are `"not_actionable"`, Sashiko records the analytics row
in `maintainer_reply_analyses` and exits the prompt-synthesis pipeline without
creating a proposal.

### 6.2 Deterministic Formal Authority & Email Authenticity Verification

Before evaluating whether a subjective claim may modify Part 1 (`maintainers/*.md`),
Sashiko deterministically computes `SenderAuthority`:

1. **Target `MAINTAINERS` Section Resolution:**
   - Resolve the affected `MAINTAINERS` sections from the touched files of the
     patch under discussion (`patchset_maintainer_sections`) and any files
     referenced by the claim via `MaintainersIndex::match_files`.
2. **Role Lookup in `MaintainersIndex` (`src/maintainers.rs`):**
   - Resolve the sender's email against the target sections:
     - `GlobalMaintainer`: Listed on a `catch_all` section (`THE REST`).
     - `SectionMaintainer`: Listed on `M:` for the target section.
     - `SectionReviewer`: Listed on `R:` for the target section.
     - `OtherSubsystemMaintainer`: Listed in `MAINTAINERS`, but only on
       unrelated sections.
     - `Unlisted`: Not in `MAINTAINERS`.
3. **Anti-Spoofing Email Authenticity Verification (`EmailAuthStatus`):**
   - Because raw `From:` headers in email can be forged, an email ingested from
     NNTP/mbox must carry cryptographic provenance before granting `M:`/`R:`
     authority automatically:
     - `VerifiedDkimOrSession`: Authenticated via Web UI JWT / API token,
       verified GitHub/GitLab webhook user identity, or a `PASS` DKIM/DMARC/ARC
       verdict from `lore.kernel.org` / trusted mail ingress matching the sender
       domain.
     - `UnverifiedHeader`: Bare `From:` header without cryptographic attestation.
       Even if the proposal passes all other checks, an `UnverifiedHeader`
       source is flagged with a prominent warning badge in the Staged Approval
       Queue so the operator verifies the lore thread before approving.

### 6.3 Stage 2: Adversarial Code Verification (`VerifyFeedbackClaimStage`)

Every extracted claim—**including issues pointed out by maintainers on patches
Sashiko reviewed**—is audited against the kernel repository using
`ToolScope::GitOnly` (`git_read_files`, `git_grep`, `git_show`, `git_blame`,
`git_log`) at `target_commit_sha`, `baseline_sha`, and `mainline`:

```rust
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ClaimVerificationOutput {
    /// "verified_true" | "verified_false" | "subjective_no_contradiction" | "safety_violation" | "unverifiable"
    pub verdict: String,
    /// Step-by-step code verification reasoning.
    pub reasoning: String,
    /// Verbatim code snippets from the kernel repository proving or disproving the claim.
    pub evidence_locations: Vec<ProofLocation>,
    /// For patch-reply misses: whether the defect pointed out by the reviewer is genuinely
    /// present in the patch at `target_commit_sha` (false if the reviewer misread the code).
    pub defect_confirmed_in_patch: bool,
    /// For objective claims, whether the claim contains any disguised subjective policy,
    /// or for subjective claims, whether it contains a falsifiable technical assertion.
    pub contains_factual_error: bool,
    /// Kernel trees where this objective claim was verified to hold (e.g. ["mainline", "linux-6.6.y"]).
    pub verified_trees: Vec<String>,
}
```

#### Strict Verification Rules
1. **Two-Level Verification for Maintainer Patch Replies:**
   - First, verify that the issue pointed out by the reviewer is genuinely a
     defect in `target_commit_sha` (`defect_confirmed_in_patch = true`). Human
     reviewers occasionally misread a diff hunk or overlook a guard outside the
     3-line diff context. If `git_read_files` at `target_commit_sha` (or the
     patch author's reply in the thread) proves the patch code is actually safe,
     the verifier returns `verdict = "verified_false"` and records the outcome
     as `author_refuted_reviewer`.
   - Second, verify that the generalized Part 2 subsystem context or Part 3 API
     contract holds across the kernel tree at `baseline_sha`.
2. **All-Callers / Symmetrical Proof Bar:**
   - If a human claims *"Helper `foo_bar()` never returns NULL"* or *"Lock `X`
     is always held when callback `Y` is invoked"*, the verifier MUST inspect
     the implementation of `foo_bar()` and `git_grep` all callers of `Y` across
     the tree.
   - If any code path in the tree contradicts the claim, the verifier MUST
     return `verdict = "verified_false"` with the disproving `code_snippet` in
     `evidence_locations`.
   - Every `verified_true` or `verified_false` verdict is structurally validated
     by `has_valid_proof_location` (requiring non-empty `file`,
     `function_or_symbol`, and verbatim `code_snippet`).
3. **Multi-Tree Scope Check for Objective Claims (Parts 2 & 3):**
   - When an objective claim is verified at `baseline_sha`, the verifier also
     checks the corresponding file/symbol at `mainline` and configured LTS
     snapshots (`git show <tree_tip_sha>:<file>`) so the resulting Part 2/3
     update is applied **only** to the `trees/<tree_id>/` snapshots where that
     code behavior actually exists.
4. **Safety Screen for Subjective Claims (Part 1):**
   - Even if a listed `M:` maintainer makes a subjective statement, the verifier
     checks two disqualifiers:
     1. **Embedded False Premise:** E.g., if someone claims a helper drops a
        refcount when it actually leaks it, that is an objective claim proven
        false by code (`verified_false`) and rejected.
     2. **Safety Bypass / Prompt Injection:** Any claim instructing Sashiko to
        ignore memory corruption, UAF, uninitialized reads, or lockdep bugs, or
        containing prompt-injection framing, returns `verdict = "safety_violation"`
        and is rejected.

### 6.4 Decision Matrix

| Claim Category | Code Verifier Verdict | Sender `MAINTAINERS` Standing | Action |
| :--- | :--- | :--- | :--- |
| **Any (Objective or Subjective)** | `verified_false` or `safety_violation` | **Any** (including `M:` Maintainer / Linus) | **REJECT** (`rejected_false_claim`) |
| **Objective (Part 2 Context / Part 3 API / Trigger Index)** | `verified_true` (with concrete code snippet proof) | **Any** (`M:`, `R:`, or `Unlisted` reviewer/author) | **ACCEPT to Staged Queue** for `verified_trees` |
| **Objective (Part 2 Context / Part 3 API)** | `unverifiable` | **Any** | **REJECT** (`rejected_unverifiable`) |
| **Subjective Policy (Part 1 Maintainers Note)** | `subjective_no_contradiction` | `GlobalMaintainer`, `SectionMaintainer` (`M:`), or `SectionReviewer` (`R:`) for target section | **ACCEPT to Staged Queue** for `maintainers/<slug>.md` |
| **Subjective Policy (Part 1 Maintainers Note)** | `subjective_no_contradiction` | `OtherSubsystemMaintainer` or `Unlisted` | **REJECT** (`rejected_insufficient_authority`) |

#### Precedence Between `M:` (Maintainer) and `R:` (Reviewer) in Part 1
- Both `M:` and `R:` in a `MAINTAINERS` section are trusted by the kernel to
  contribute to that section's `maintainers/<slug>.md`.
- However, each rule in `maintainers/<slug>.md` records an HTML comment or
  structured attribution tag (`<!-- role: M, author: user@kernel.org, msg: <id> -->`).
- If a newly proposed subjective rule from an `R:` (Reviewer) contradicts an
  existing rule authored by an `M:` (Maintainer) of the same section, the
  synthesizer flags the conflict (`has_authority_conflict = true`) and prevents
  overwriting the `M:` rule without explicit `M:` or operator confirmation.
  Conversely, an `M:` (Maintainer) can update or supersede an earlier `R:` rule.

### 6.5 Stage 3: Prompt Diff Synthesis & Sanitization (`SynthesizePromptUpdateStage`)

For claims that pass the Decision Matrix:
1. Reads the current target `.md` file (e.g., `maintainers/networking-general.md`,
   `trees/mainline/contexts/net/core/skbuff.md`, or `trees/mainline/apis/skb-buffer.md`).
2. Synthesizes a minimal, canonical, third-person Markdown update integrated
   into the appropriate section (avoiding duplicate rules). If the trigger was a
   `false_dismissal`, includes an explicit **Do Not Dismiss When...** note
   addressing the flawed assumption from `dismissal_flaw_diagnosis`.
3. **Deterministic Security & Boundary Validators:**
   - **No Template Directives:** Rejects any output containing `@include`,
     `@includes`, or `{{` / `}}` so feedback can never inject `PromptTemplate`
     directives (`prompts/sashiko/prompt-injection.md`).
   - **No Verbatim Untrusted Prose:** The rule is re-expressed by the synthesizer
     in concise technical prose.
   - **Part 1 vs. Part 2/3 Purity Check:** Validates that `maintainers/*.md`
     updates do not introduce struct/function implementation summaries and
     `contexts/` / `apis/` updates do not introduce subjective style rules.

---

## 7. Staged Approval Queue, Miss Analytics & Database Schema

Because the system uses a **Staged Approval Queue**, both differential maintainer
reply analyses and validated prompt update proposals are stored in SQLite for
operator inspection, analytics, and one-click approval in the Sashiko Web UI or
CLI (`sashiko-cli`).

### 7.1 Database Schema (`src/migrations/014_prompt_feedback_queue.sql`)

```sql
-- Stores every issue extracted from a maintainer/reviewer reply on a patch
-- that Sashiko reviewed, enabling recall and gap analytics across subsystems.
CREATE TABLE IF NOT EXISTS maintainer_reply_analyses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_uuid TEXT NOT NULL UNIQUE,
    source_message_id TEXT NOT NULL,
    patchset_id INTEGER NOT NULL,
    patch_id INTEGER,
    review_id INTEGER,
    author_email TEXT NOT NULL,
    maintainer_role TEXT NOT NULL DEFAULT 'unlisted',
    matched_maintainer_sections_json TEXT NOT NULL DEFAULT '[]',

    -- Summary and quote of the issue pointed out by the human reviewer
    maintainer_issue_summary TEXT NOT NULL,
    source_quote TEXT NOT NULL,
    target_locations_json TEXT NOT NULL DEFAULT '[]',

    -- Differential comparison against Sashiko's review of the same patch:
    -- 'already_caught': Sashiko reported this issue in `findings`
    -- 'false_dismissal': A Sashiko stage raised it (`C*`/`D*`), but Sashiko dismissed it
    -- 'selection_gap': Covered by an existing Part 1/2/3 guide that was not selected
    -- 'unspotted_miss': Never raised by any Sashiko stage and not in existing guides
    -- 'author_refuted_reviewer': Code verification or author reply proved patch is safe
    differential_outcome TEXT NOT NULL
        CHECK (differential_outcome IN (
            'already_caught',
            'false_dismissal',
            'selection_gap',
            'unspotted_miss',
            'author_refuted_reviewer'
        )),

    -- Link to Sashiko's internal review trace when applicable
    matched_finding_id TEXT,
    dismissed_in_stage TEXT,
    matched_dismissed_id TEXT,
    dismissal_flaw_diagnosis TEXT,

    -- Fine-grained taxonomy of what type of issue the maintainer pointed out
    missed_issue_taxonomy TEXT NOT NULL
        CHECK (missed_issue_taxonomy IN (
            'subjective_architecture_or_direction',
            'subjective_process_or_tree_routing',
            'subjective_style_or_convention',
            'subsystem_state_or_lifecycle_invariant',
            'subsystem_locking_or_execution_context',
            'api_contract_or_ownership_pitfall',
            'hardware_abi_or_spec_constraint',
            'patch_specific_logic_bug',
            'other'
        )),

    created_at INTEGER NOT NULL,
    FOREIGN KEY(patchset_id) REFERENCES patchsets(id) ON DELETE CASCADE,
    FOREIGN KEY(patch_id) REFERENCES patches(id) ON DELETE SET NULL,
    FOREIGN KEY(review_id) REFERENCES reviews(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_maintainer_reply_analyses_outcome
    ON maintainer_reply_analyses(differential_outcome, missed_issue_taxonomy, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_maintainer_reply_analyses_patchset
    ON maintainer_reply_analyses(patchset_id);
CREATE INDEX IF NOT EXISTS idx_maintainer_reply_analyses_patch
    ON maintainer_reply_analyses(patch_id);
CREATE INDEX IF NOT EXISTS idx_maintainer_reply_analyses_review
    ON maintainer_reply_analyses(review_id);

CREATE TABLE IF NOT EXISTS prompt_update_proposals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    proposal_uuid TEXT NOT NULL UNIQUE,

    -- Status lifecycle:
    -- 'pending_approval': Passed code verification / MAINTAINERS check, awaiting operator approval
    -- 'approved': Applied and committed to the overlay Git repository
    -- 'rejected_by_operator': Manually rejected by an operator
    -- 'rejected_false_claim': Automatically rejected because kernel code disproved the claim
    -- 'rejected_insufficient_authority': Automatically rejected (subjective claim by non-maintainer)
    -- 'rejected_unverifiable': Automatically rejected (objective claim without code proof)
    -- 'rejected_safety_violation': Automatically rejected (attempted safety bypass or injection)
    status TEXT NOT NULL DEFAULT 'pending_approval'
        CHECK (status IN (
            'pending_approval',
            'approved',
            'rejected_by_operator',
            'rejected_false_claim',
            'rejected_insufficient_authority',
            'rejected_unverifiable',
            'rejected_safety_violation'
        )),

    -- Target prompt classification: 'maintainer_notes' | 'code_context' | 'api_guide' | 'trigger_index'
    prompt_part TEXT NOT NULL
        CHECK (prompt_part IN ('maintainer_notes', 'code_context', 'api_guide', 'trigger_index')),

    -- Target tree ID (NULL for Part 1 maintainer_notes; e.g. 'mainline', 'linux-6.6.y' for Parts 2 & 3)
    tree_id TEXT,

    -- Relative path within the kernel prompt root (e.g. 'maintainers/bpf.md' or 'trees/mainline/apis/rcu.md')
    target_relpath TEXT NOT NULL,

    -- Provenance of the human feedback or maintainer patch reply
    source_kind TEXT NOT NULL CHECK (source_kind IN ('mailing_list', 'forge', 'ui_api', 'periodic_sync')),
    source_message_id TEXT,
    source_patchset_id INTEGER,
    source_patch_id INTEGER,
    source_review_id INTEGER,
    reply_analysis_id INTEGER,
    author_email TEXT NOT NULL,
    author_display TEXT,
    email_auth_status TEXT NOT NULL DEFAULT 'unverified_header'
        CHECK (email_auth_status IN ('verified_dkim_or_session', 'unverified_header')),

    -- Formal MAINTAINERS standing resolved at evaluation time
    maintainer_role TEXT NOT NULL DEFAULT 'unlisted'
        CHECK (maintainer_role IN (
            'global_maintainer',
            'section_maintainer',
            'section_reviewer',
            'other_subsystem_maintainer',
            'unlisted'
        )),
    matched_maintainer_sections_json TEXT NOT NULL DEFAULT '[]',

    -- Differential miss metadata & Verification Evidence
    differential_outcome TEXT,
    missed_issue_taxonomy TEXT,
    dismissed_in_stage TEXT,
    matched_dismissed_id TEXT,
    claim_category TEXT NOT NULL,
    claim_summary TEXT NOT NULL,
    source_quote TEXT NOT NULL,
    verification_verdict TEXT NOT NULL,
    verification_reasoning TEXT NOT NULL,
    verified_on_sha TEXT,
    evidence_locations_json TEXT NOT NULL DEFAULT '[]',
    has_authority_conflict INTEGER NOT NULL DEFAULT 0,

    -- Proposed Markdown content & unified diff
    base_blob_sha TEXT,
    proposed_markdown TEXT,
    unified_diff TEXT,
    applied_commit_sha TEXT,

    -- Audit fields
    reviewed_by TEXT,
    reviewed_at INTEGER,
    review_note TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,

    FOREIGN KEY(source_patchset_id) REFERENCES patchsets(id) ON DELETE SET NULL,
    FOREIGN KEY(source_patch_id) REFERENCES patches(id) ON DELETE SET NULL,
    FOREIGN KEY(source_review_id) REFERENCES reviews(id) ON DELETE SET NULL,
    FOREIGN KEY(reply_analysis_id) REFERENCES maintainer_reply_analyses(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_prompt_proposals_status
    ON prompt_update_proposals(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_part_tree
    ON prompt_update_proposals(prompt_part, tree_id, status);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_outcome_taxonomy
    ON prompt_update_proposals(differential_outcome, missed_issue_taxonomy, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_message_id
    ON prompt_update_proposals(source_message_id);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_source_patchset
    ON prompt_update_proposals(source_patchset_id);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_source_patch
    ON prompt_update_proposals(source_patch_id);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_source_review
    ON prompt_update_proposals(source_review_id);
CREATE INDEX IF NOT EXISTS idx_prompt_proposals_reply_analysis
    ON prompt_update_proposals(reply_analysis_id);
```

### 7.2 HTTP API & CLI Endpoints

| Endpoint | Capability Required | Description |
| :--- | :--- | :--- |
| `GET /api/prompts/proposals` | Operator (`[server.acl].admins` or local token) | Paginated list of prompt update proposals filtered by `status`, `prompt_part`, `tree_id`, `differential_outcome`, or `missed_issue_taxonomy`. |
| `GET /api/prompts/proposal?id=<uuid>` | Operator | Full proposal details including `unified_diff`, `evidence_locations`, `MAINTAINERS` verification, and the linked `maintainer_reply_analyses` trace (e.g., which Sashiko stage dismissed the concern). |
| `GET /api/prompts/miss-analytics` | Operator | Aggregated telemetry on maintainer replies to Sashiko-reviewed patches: breakdown by `differential_outcome` (`already_caught` vs `false_dismissal` vs `selection_gap` vs `unspotted_miss`), by `missed_issue_taxonomy` (Part 1 subjective policy vs Part 2 code context vs Part 3 API contract), by `dismissed_in_stage`, and by `MAINTAINERS` section. |
| `POST /api/prompts/proposal/approve` | Operator | Applies `proposed_markdown` to the overlay Git repo, commits with provenance trailers, and transitions status to `approved`. |
| `POST /api/prompts/proposal/reject` | Operator | Transitions status to `rejected_by_operator` with optional `review_note`. |
| `POST /api/prompts/feedback` | Authenticated (`BugAccess::Comment` / `Manage` or Operator) | Submit explicit feedback on a review finding or subsystem/API prompt to trigger the evaluation pipeline. |

When an operator approves a proposal (`POST /api/prompts/proposal/approve`),
Sashiko acquires the process-wide `overlay_git_lock: Arc<tokio::sync::Mutex<()>>`
shared between the API handler and the background prompt-sync worker to prevent
concurrent `.git/index.lock` collisions on the local overlay repository, writes
`proposed_markdown` atomically, and commits the change using `git_cmd::in_dir_async`
with a structured commit message:

```text
prompts(apis/mainline): document skb_unshare NULL return semantics

Update trees/mainline/apis/skb-buffer.md to clarify that skb_unshare()
frees the original skb via kfree_skb() on allocation failure and returns
NULL, so callers must not free the input pointer again.

Prompt-Part: api_guide
Tree-Id: mainline
Verified-On-Sha: a1b2c3d4e5f60123456789abcdef0123456789ab
Source-Message-Id: <202610121530.xyz@lore.kernel.org>
Feedback-Author: Jane Doe <jane@example.org>
Maintainer-Role: unlisted
Verification-Verdict: verified_true
Approved-By: operator@sashiko.dev
Signed-off-by: Sashiko Prompt Bot <bot@sashiko.dev>
```

---

## 8. Step-by-Step Implementation Plan

1. **Commit 1 — Database Schema (`014_prompt_feedback_queue.sql`):**
   - Add `maintainer_reply_analyses` and `prompt_update_proposals` tables and
     CRUD methods in `src/db.rs`.
2. **Commit 2 — Reply Ingestion Filter & Target-Patch Resolver:**
   - Implement deterministic tag-only filtering, `ResolveReplyTargetPatch`
     (`in_reply_to` / `references_hdr` $\rightarrow$ `patch_id` + `review_id`),
     and the sub-thread settling window (`reply_settle_seconds`).
3. **Commit 3 — Maintainer Reply Differential Analyzer (`CompareMaintainerReplyStage`):**
   - Compare maintainer review points against Sashiko's stored `findings`,
     `dismissed_concerns`, `review_map`, and `selected_guides` to classify
     `differential_outcome` (`already_caught`, `false_dismissal`,
     `selection_gap`, `unspotted_miss`, `author_refuted_reviewer`) and
     `missed_issue_taxonomy`.
4. **Commit 4 — Adversarial Claim Verifier & Epistemic Decision Gate:**
   - Implement `ExtractFeedbackClaimsStage`, `SenderAuthority` (`M:` vs `R:`) +
     `EmailAuthStatus` verification, `VerifyFeedbackClaimStage`
     (`ToolScope::GitOnly`), and `SynthesizePromptUpdateStage`.
5. **Commit 5 — Staged Approval Queue & Miss Analytics API, CLI, and Web UI:**
   - Add `/api/prompts/proposals`, `/api/prompts/proposal/approve`,
     `/api/prompts/proposal/reject`, `/api/prompts/miss-analytics`, and
     `/api/prompts/feedback` endpoints, overlay Git commit writer, and Web UI
     tabs.
