# General Subjective Kernel Patch Review Notes

Apply only when a subjective design, API, and commit-message review is explicitly requested:
- **Grammar and typo noise:** Never flag isolated minor grammar issues (such as `its` vs `it's`) unless a concentration of errors makes a commit message or comment genuinely difficult to understand.
- **Simplicity and code duplication:** Compare proposed code complexity against surrounding subsystem code. Check both for duplicated logic inside the patch itself and for reimplementations of existing kernel helpers.
- **API design and encapsulation:**
  - When new APIs are introduced, verify naming, argument ordering, locking contracts, and allocation/free ownership consistency with adjacent kernel APIs.
  - When existing APIs are consumed, check that callers do not reach into private/internal struct fields that violate the abstraction boundary.
- **Commit message quality:** Verify that the commit title and changelog accurately reflect the diff, explain *why* the change is needed (not merely *what* changed), and include relevant user-visible or architectural context without unnecessary verbosity.
