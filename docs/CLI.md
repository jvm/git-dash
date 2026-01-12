# CLI Update Mode Specification

This document specifies the proposed CLI update mode for git-dash. The goal is to
use a single tool that behaves as a CLI when update flags are provided and falls
back to the existing TUI when no update flags are present.

## Mode Selection

- TUI mode: no update flags provided (e.g., `git-dash` or `git-dash <path>`).
- CLI mode: any update flag is present (`--pull` or `--push`).
- A scan root path is still accepted in both modes (defaults to CWD).

## CLI Usage

```
git-dash [OPTIONS] [PATH]

Update options (CLI mode):
  --pull            Pull updates (fast-forward only).
  --push            Push updates.
  --repo <name>     Target a single repo by folder name.
  --dry-run         Show what would run without executing git commands.
  --dirty <mode>    Handling for dirty repos: skip | allow | stash (default: skip).
```

Defaults:
- If neither `--pull` nor `--push` is provided in CLI mode, perform both in order:
  pull then push.
- If `--repo` is not provided, all valid git repos under the scan root are used.
- `--dirty=skip` is the default.
- Implicit confirmation: presence of update flags means no interactive prompt.

## Target Selection

- Only valid git repos are included.
- `--repo <name>` matches a top-level folder name exactly.
- If the target repo is not found, exit with failure and a message.
- Only one target repo can be selected at a time.

## Operation Order

- Combined operations always run in fixed order: pull then push.

## Dirty Repo Handling

- skip (default): do not run any update in that repo; report as skipped.
- allow: run updates even if dirty; failures handled by git.
- stash: stash uncommitted changes (including untracked), run updates, then pop.
  If stash pop conflicts, mark as failed and leave stash intact.

TUI behavior should follow the same rules when triggering pull/push for the
selected repo.

## Failure Handling

- In all-repos mode, failures do not stop processing other repos.
- A summary is printed at the end with counts for success, skipped, and failed.
- The exit code is non-zero if any repo failed or was skipped.

## Dry Run

- Print the resolved repo list and the exact git commands that would run per repo.
- Do not execute any git commands.

## Output

- Default: one line per repo with status (OK, SKIP, FAIL) and a short reason.
- `--verbose` and `--quiet` are optional extensions if needed later.

## Shared Logic

- CLI and TUI should use the same internal update runner:
  - repo filtering
  - dirty handling
  - remote validation
  - pull/push execution
  - result aggregation and summary formatting
