# git-dash — Project Specification

## 1. Overview

git-dash is a fast, local-first TUI (terminal user interface) tool written in Rust using **ratatui**. It scans the current directory for Git repositories and provides a keyboard-driven interface to inspect repository status and perform common actions such as pull and push.

The primary goal is to reduce friction when working with multiple local repositories in parallel, without introducing server components, background daemons, or network dependencies beyond Git itself.

---

## 2. Goals

### Primary Goals
- Discover Git repositories under the current working directory
- Display repository state at a glance (branch, dirty state, ahead/behind)
- Enable quick **pull** and **push** actions via keyboard
- Remain fast and responsive even with dozens of repositories
- Require zero configuration by default

### Non-Goals
- Full Git porcelain replacement
- Commit creation, rebasing, or conflict resolution
- Remote repository browsing
- Authentication management beyond what Git already provides

---

## 3. Target Users

- Engineers working with mono-repo-adjacent or multi-repo setups
- Developers managing multiple services or libraries locally
- Power users who prefer keyboard-driven workflows
- Rust and Git users on macOS and Linux (Windows optional later)

---

## 4. Core Use Cases

1. Scan a directory and list all Git repositories
2. See which repositories have uncommitted changes
3. Identify repositories that are ahead or behind their upstream
4. Pull latest changes for a selected repository
5. Push local commits for a selected repository
6. Batch pull or push multiple repositories (future)

---

## 5. Functional Requirements

### Repository Discovery
- Recursively scan the current directory
- Identify repositories by presence of a `.git` directory or file
- Ignore nested repositories by default (configurable later)

### Repository Status
For each repository, display:
- Repository name
- Current branch
- Dirty/clean working tree (with color coding: yellow for dirty, cyan for clean)
- Ahead/behind counts vs upstream (if configured)
- Change summary (counts by type: M:2 A:1 D:1 format)
- Remote URL (simplified display: github.com/user/repo)
- Last fetch timestamp (human-readable: 5m, 2h, 3d)
- Error messages inline when Git operations fail

### Actions
- Pull (fast-forward only by default)
- Push (current branch)
- Refresh status

All actions must:
- Be non-blocking to the UI
- Surface success or failure clearly
- Never modify repositories silently
- Validate remote configuration before attempting push/pull operations
- Require explicit user confirmation via y/n prompt

---

## 6. User Interface (TUI)

### Layout
- Header: tool name, current path, progress bar during scan
- Main list: 7-column table of repositories with status
- Footer: keybindings and contextual status messages

### Interaction Model
- Keyboard-only navigation
- Vim-style or common TUI keybindings
- Confirmation prompts for destructive or network actions (y/n/Esc)

### Keybindings
Navigation:
- `j` / `k` or `↓` / `↑`: Move selection up/down
- `PageDown` / `PageUp`: Jump 10 repositories at a time

Actions:
- `p`: Pull selected repository (prompts for confirmation)
- `u`: Push selected repository (prompts for confirmation)
- `r`: Refresh status for all repositories

Confirmation prompts:
- `y`: Confirm action
- `n` or `Esc`: Cancel action

Exit:
- `q`: Quit application
- `Ctrl+C`: Force quit

### Progress Indicators
- Animated progress bar during initial repository scan
- Two-phase scanning: 40% for discovery, 60% for status fetching
- Real-time progress updates every 20 directories

### Accessibility
- Color-blind friendly palette (yellow for dirty, cyan for clean)
- Clear ASCII indicators (dirty *, clean .)
- High contrast text for all UI elements

---

## 7. Technical Architecture

### Language & Frameworks
- Rust (stable)
- ratatui for TUI rendering
- crossterm for terminal handling

### Git Integration
- Prefer invoking the system `git` binary
- Avoid libgit2 unless strictly necessary
- Parse porcelain output for status information

### Concurrency Model
- Background worker thread for Git operations
- Parallel status fetching using scoped threads for improved performance
- Message-passing (mpsc channels) to update UI state
- No shared mutable state across threads

### Module Organization
- `main.rs`: Application entry point and event loop
- `app.rs`: Application state and logic
- `discovery.rs`: Repository discovery and gitdir resolution
- `git.rs`: Git command execution with timeouts
- `logger.rs`: Debug logging functionality
- `status.rs`: Git status parsing and formatting
- `ui.rs`: TUI rendering with ratatui
- `worker.rs`: Background worker and parallel operations

---

## 8. Error Handling & Safety

- Never assume upstreams exist (validated before push/pull operations)
- Gracefully handle detached HEAD (shown as "DETACHED" branch)
- Surface Git errors verbatim in the UI table
- Timeouts for long-running Git operations (30s for operations, 5s for status)
- Error states clearly distinguished (timeout vs error in change summary)
- Thread-safe error collection during parallel status fetching

---

## 9. Command-Line Interface

### Usage
```
git-dash [OPTIONS] [PATH]
```

### Arguments
- `PATH`: Optional directory to scan (defaults to current directory)

### Options
- `-d, --debug`: Enable debug logging to `git-dash-debug.log`
- `-h, --help`: Print help information

### Debug Logging
When enabled with `--debug`, logs include:
- Timestamp with millisecond precision
- Repository scan progress
- Git command execution times
- Success/failure status for all operations
- Performance metrics for status fetching

---

## 10. Configuration (Future)

Initial version:
- No configuration file

Planned:
- Ignore patterns
- Default pull strategy
- Batch operation toggles

---

## 11. Performance Targets & Optimizations

### Target Performance
- Initial scan under 200 ms for ~50 repositories
- UI redraws under 16 ms (60 FPS capable)
- Git operations executed lazily and on demand

### Implemented Optimizations
- Parallel status fetching using scoped threads (all repos checked concurrently)
- Progress updates batched every 20 directories to reduce UI overhead
- Porcelain v2 format parsing for efficient status extraction
- Non-blocking UI during all Git operations
- Nested repository detection stops directory traversal early

---

## 12. Testing

### Unit Tests (14 total)
Located in `src/status.rs`:
- Git status line parsing (including paths with spaces)
- Status code transformation
- Change summarization
- Remote URL simplification (git@, https://, ssh://)
- Age formatting for last fetch timestamps
- Repository name extraction

### Integration Tests (3 total)
Located in `tests/repo_discovery.rs`:
- Multi-directory repository discovery
- Nested repository handling (discovery stops at outer repo)
- Gitdir file handling (worktrees and submodules)

---

## 13. Security Considerations

- No credential handling
- No network calls outside Git
- No telemetry or data collection

---

## 14. Distribution

- Single static binary where possible
- Installable via `cargo install`
- GitHub releases for macOS and Linux
- **Homebrew support (macOS)**
  - Provide an official Homebrew tap (e.g. `brew install <tap>/git-dash`)
  - Formula should install prebuilt binaries when available
  - Formula must verify checksums and support Apple Silicon and Intel

---

## 15. Roadmap

### v0.1 (Completed)
- ✅ Repository discovery with nested repo detection
- ✅ Comprehensive status view (7 columns)
- ✅ Pull / push actions with confirmation prompts
- ✅ Remote validation before operations
- ✅ Progress indicators during scanning
- ✅ Parallel status fetching for performance
- ✅ Debug logging option
- ✅ Comprehensive test suite (17 tests)
- ✅ Homebrew tap support (personal tap: jvm/tap)

### v0.2 (Completed)
- ✅ Search and filter repositories
- ✅ Sorting options (name/status/ahead-behind/last fetch)
- ✅ Help screen with keybindings
- ✅ Header stats (repo/dirty/ahead/behind counts)
- ✅ Empty state handling
- ✅ Colorized change summary display
- ✅ Scroll hints for long lists

### v0.3 (Planned)
- TBD

---

## 16. Design Decisions & Answers

### Directory Scanning Depth
- **Decision**: Unlimited depth, but stop at nested repositories
- **Rationale**: Users may have deep directory structures, but nested repos indicate a boundary

### Nested Repositories
- **Decision**: Hidden (discovery stops at outer repository)
- **Rationale**: Prevents duplicate operations and confusion; users should manage inner repos separately

### Fetch Strategy
- **Decision**: Explicit only (user must trigger with `r` refresh)
- **Rationale**: Avoids unexpected network calls; user controls when network operations occur

### Path Handling
- **Decision**: Support paths with spaces in porcelain parsing
- **Rationale**: Modern projects often have spaces in filenames
