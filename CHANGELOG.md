# Changelog

## 0.2.0 - 2026-11-15

### Added
- `replica new --ref <branch>`: create a workspace from a specific branch. If the branch's lockfile matches the base's recorded hash, the workspace is cloned directly. If it differs, Replica clones the base, checks out the branch, and runs the setup command (an incremental install), reporting which path it took.
- `replica new --require-fresh`: refuse to create a workspace from a stale base (exit 2).
- `replica base refresh` fast path: clonefiles the base, fetches origin, resets to `origin/HEAD`, and re-runs setup only if a lockfile changed. Falls back to a full rebuild on failure.
- `replica base check`: detect build output that breaks when cloned (absolute paths in venvs, `node_modules/.bin`, `.tsbuildinfo`).
- Concurrency tests: `tests/concurrency.rs` runs 10 simultaneous `replica new`/use/`rm` cycles with zero SQLite busy errors.
- Git behavior tests: `tests/git_workspace.rs` verifies a workspace commits and pushes without touching the base, and refresh picks up new commits.
- Parallel safety: `replica new` and `replica gc` are safe to run at once. Each workspace is created under a lease (`.replica/lease`) so `gc` never removes a workspace that is still being set up or is in use.
- Benchmark suite extended: concurrency (N = 1, 5, 10), pnpm global store, `cp -c`, `git worktree`, full metrics (P50, P95, min, max), and recorded runs in `benchmarks/results/`.
- Agent skills in `skills/`: `replica`, `replica-base`, `replica-maintenance`, `replica-dev`, `replica-benchmarks`, `replica-release`.

### Changed
- `replica setup` installs the SKILL.md skill, replacing `skill.toml` and `instructions.md`.
- `replica setup` keeps existing replaced configs unless `--force` is given. `--force` always backs up the original first.
- `replica setup` no longer duplicates appended configs when run again.
- Base metadata records the repo and setup command, and the database migrates to add them.
- README rewritten: git workflow in workspaces, base refresh, disk cost after changes, measured file counts, and a full account of the benchmark numbers.
- Benchmark docs now state what was measured. Clone disk cost is about 1.4 MB of APFS metadata per clone, not zero.

### Fixed
- Workspace IDs are now 16 hex chars (8 time-based + 8 random) instead of 8. The old scheme could produce identical IDs for two workspaces created in the same process within a few hundred nanoseconds, which would cause a data-loss collision on the workspace directory.
- `scripts/install.sh` file mode corrected.
- Removed an `unwrap` from `replica base check`'s `pyvenv.cfg` parsing.

### Notes
- Homebrew formula is not part of the release yet (see HOMEBREW.md).
- Known open items: orphan sweep, concurrency transactions, post-clone cleanup flag (see docs/ROADMAP.md).

## 0.1.0 - 2026-11-09

### Added
- `replica help [topic]`: a guided overview, with topics quickstart, workspaces, base, safety, agents, env, and exit-codes. It works before `REPLICA_ROOT` exists.
- Clear command output: status lines (`✓ ! ✗ ·`) on stderr, aligned tables for `list`, a usage bar for `stats`, and `hint:` lines on errors. Color is used only on a TTY, and `NO_COLOR` turns it off.
- `--json` for `list`, `stats`, and `base status`.
- Typed exit codes: 2 (no base or stale with `--require-fresh`), 3 (cross-volume), 4 (disk full or byte cap). They are used consistently and documented.
- `replica base refresh` works. It rebuilds from the repo and setup command recorded at build time.
- `replica env` and `replica touch` reject unknown ids with exit 1.
- Benchmark suite in `benchmarks/`: hyperfine-based, with untimed setup and cleanup, a pinned fixture project, and a machine-readable report. `make bench` runs it.
- Agent skills in `skills/`, in SKILL.md format for the skills.sh CLI: `replica`, `replica-base`, `replica-maintenance`, `replica-dev`, `replica-benchmarks`, `replica-release`.
- `make help`, `make bench`, and `make skills-check`.
- `scripts/release.sh` for building and publishing a release. The Homebrew formula is not part of the release yet (see HOMEBREW.md).

- `replica base refresh` fast path: clonefiles the base, fetches origin, resets to `origin/HEAD`, and re-runs setup only if a lockfile changed. Falls back to a full rebuild on failure.
- Integration tests for git behavior: a workspace commits and pushes without touching the base, and refresh picks up new commits.

### Changed
- Dependencies: `sha2` 0.11, `toml` 1.2, `rusqlite` 0.41, `dirs` 7, and current GitHub Actions versions. Lockfile hashing hex-encodes the digest explicitly.
- README rewritten: git workflow in workspaces, base refresh, disk cost after changes, measured file counts, and an account of the benchmark numbers.
- `replica setup` installs the `SKILL.md` skill, replacing `skill.toml` and `instructions.md`. It no longer writes the placeholder `skills.sh`.
- `replica setup` no longer duplicates appended configs when run again.
- `replica setup` keeps existing replaced configs unless `--force` is given. `--force` always backs up the original first.
- Base metadata records the repo and setup command, and the database migrates to add them.
- Benchmark docs now state what was measured. Clone disk cost is about 1.4 MB of APFS metadata per clone, not zero.
- Default GitHub owner is `gabrielvfonseca`, matching the git remote.

### Fixed
- A missing `REPLICA_ROOT` produces a clear error with a hint, not a raw `statfs` failure.
- Exit codes match the documentation. Code 4 was documented but never returned.
- `replica rm` says when a workspace is already gone. It keeps going past a failed removal, reports it, and exits 1 at the end.
- Removed an `unwrap` from `replica base check`'s `pyvenv.cfg` parsing, so non-test code follows the no-unwrap rule in AGENTS.md.

### Notes
- The current benchmark suite has not yet produced a run on an idle machine. See docs/BENCHMARKS.md.
- `Cargo.lock` on main had a duplicate `thiserror` entry that stopped cargo from parsing it. It is repaired.
