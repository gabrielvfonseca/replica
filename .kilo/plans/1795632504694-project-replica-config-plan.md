# Project-Level `.replica/` Configuration Plan

## Overview
Add a per-project `.replica/` directory in the git repository root that stores comprehensive project configuration and settings. This enables teams to share workspace defaults, base build settings, agent configurations, and GC parameters while allowing machine-specific overrides.

---

## Goals
- **G1**: Project-level config directory `.replica/` at git repo root
- **G2**: New `replica init` command for interactive project setup
- **G3**: Wire project config into `replica new`, `replica base build`, `replica gc`, `replica setup`
- **G4**: Support template + local override pattern (track template, gitignore local)
- **G5**: Maintain backward compatibility - all flags still override project config

---

## Non-Goals
- Daemon or background config watching
- Remote config sync (beyond git)
- Per-user global config (already exists at `~/.config/replica/agents.json`)

---

## Design Decisions

### Directory Structure
```
<project-root>/
├── .replica/
│   ├── config.json          # Tracked in git (team defaults)
│   ├── config.local.json    # Gitignored (machine-specific overrides)
│   ├── skills/              # Project-specific agent skills (optional)
│   │   └── <agent>/SKILL.md
│   └── templates/           # Project-specific config templates (optional)
│       └── <agent>.md
└── .gitignore               # Add: .replica/config.local.json
```

### Config Schema (JSON)
```json
// .replica/config.json (tracked)
{
  "base": {
    "repo": "https://github.com/org/repo.git",
    "setup_cmd": "npm ci",
    "default_branch": "main"
  },
  "workspace": {
    "default_ttl": "2h",
    "default_name_template": "task-{{timestamp}}",
    "post_clone_hooks": ["direnv allow", "pnpm install"],
    "require_fresh_default": false
  },
  "gc": {
    "high_water_mark": 80.0,
    "low_water_mark": 60.0,
    "ttl_override": "1d"
  },
  "agents": {
    "claude": { "append": ".replica/templates/claude.md" },
    "cursor": { "replace": ".replica/templates/cursorrules" },
    "copilot": { "append": ".replica/templates/copilot-instructions.md" }
  },
  "skills": {
    "enabled": true,
    "custom_skills_dir": ".replica/skills"
  }
}
```

```json
// .replica/config.local.json (gitignored)
{
  "base": {
    "repo": "https://github.com/myfork/repo.git"
  },
  "workspace": {
    "default_ttl": "4h",
    "post_clone_hooks": ["direnv allow", "pnpm install", "local-script.sh"]
  },
  "gc": {
    "high_water_mark": 70.0
  }
}
```

### Config Resolution Order (highest priority wins)
1. CLI flags (explicit user override)
2. `config.local.json` (machine-specific)
3. `config.json` (project defaults)
4. Hardcoded defaults (current behavior)

---

## Implementation Plan

### Phase 1: Core Config Infrastructure

#### 1.1 Add Config Module (`src/config.rs`)
- `ProjectConfig` struct with all sections
- `load_project_config(project_root: &Path)` → reads both files, merges with precedence
- `find_project_root(start: &Path)` → walks up to find `.replica/config.json` or git root
- `ConfigError` with helpful hints

#### 1.2 Add Config to Workspace Context
- Modify `main.rs` to detect project root early (before `REPLICA_ROOT` checks)
- Pass `ProjectConfig` to commands that need it
- Commands without project root (help, setup) work as before

#### 1.3 Update `replica new`
- Read project config for defaults: `ttl`, `name`, `post_clone`, `require_fresh`
- CLI flags override config values
- If `--ref` not given but `base.default_branch` set, use it

#### 1.4 Update `replica base build`
- Read project config for `repo` and `setup_cmd`
- CLI flags override config values
- Warn if neither config nor flags provide required values

### Phase 2: `replica init` Command

#### 2.1 Add `Init` Subcommand to CLI
```rust
Cmd::Init {
    /// Project root (default: current directory)
    #[arg(long)]
    path: Option<PathBuf>,
    /// Non-interactive: use defaults, don't prompt
    #[arg(long)]
    yes: bool,
    /// Force overwrite existing config
    #[arg(long)]
    force: bool,
}
```

#### 2.2 Interactive Setup Flow
1. Detect project root (git repo or current dir)
2. Check for existing `.replica/` - prompt if exists (unless `--force`)
3. Prompt for:
   - Base repo URL (with git remote detection)
   - Setup command (detect from package.json, Cargo.toml, Makefile)
   - Default TTL (30m/2h/1d)
   - Post-clone hooks (suggest common ones)
   - Agent templates to include
6. Write `.replica/config.json` and `.replica/config.local.json`
7. Add `.replica/config.local.json` to `.gitignore`
6. Optionally create agent template files in `.replica/templates/`

#### 2.3 Template Files
- Generate `.replica/templates/claude.md`, `cursorrules`, etc. from built-in templates
- Allow customization during init

### Phase 3: Agent Config Integration

#### 3.1 Extend `replica setup`
- Read project config `[agents]` section
- For each agent, append/replace project-specific template
- Install project-specific skills from `.replica/skills/`
- Merge with global agent config (global first, then project)

#### 3.2 Per-Project Skill Installation
- Skills in `.replica/skills/<agent>/SKILL.md` installed alongside global skills
- `replica setup` detects and installs them

### Phase 4: GC Integration

#### 4.1 Project-Aware GC
- `replica gc` reads project config if run from within a project
- Project `high_water_mark`/`low_water_mark` override global defaults
- Project `ttl_override` affects expiry calculation for workspaces from this project

#### 4.2 Workspace-to-Project Link
- Store `project_root` in workspace DB row (optional, for GC awareness)
- Or: GC uses project config only when invoked from project directory

### Phase 5: Polish & Documentation

#### 5.1 CLI Help Updates
- Add `init` to command list
- Update `new`, `base build`, `gc`, `setup` help with config references
- New help topic: `replica help config`

#### 5.2 Documentation
- Update `docs/CLI.md` with `init` command and config file reference
- Add `docs/CONFIG.md` with full schema and examples
- Update `replica help config` topic

#### 5.3 Tests
- Unit: config loading, merging, precedence
- Integration: `init` creates valid config, `new` uses defaults, `base build` uses config
- Edge cases: missing config, partial config, local overrides, gitignore handling

---

## Affected Files

### New Files
- `src/config.rs` - Config loading, merging, validation
- `src/init.rs` - `replica init` command implementation
- `docs/CONFIG.md` - Configuration reference
- `.replica/` template files (embedded in binary)

### Modified Files
- `src/main.rs` - Add `Init` command, project root detection, pass config to commands
- `src/clone.rs` / `new_workspace` - Use project config defaults
- `src/store.rs` - Potentially add `project_root` to workspace row
- `src/gc.rs` - Read project config for water marks
- `src/setup.rs` - Read project agent config, install project skills
- `src/help.rs` - Add `init` command, `config` topic
- `docs/CLI.md` - Document `init` and config file
- `Cargo.toml` - No new deps needed (uses existing `serde_json`, `dirs`, `serde`)

---

## Migration & Rollout

### Backward Compatibility
- All existing commands work without project config
- CLI flags always override config
- No database migration needed (optional `project_root` column can be added later)

### Rollout Steps
1. Implement `config.rs` and `init.rs` behind feature flag or in `init` command only
2. Wire `new` and `base build` to use config
3. Wire `setup` and `gc` to use config
4. Add documentation and help
5. Remove feature flag

---

## Open Questions

1. **Project root detection**: Use git root? `.replica/` presence? Config file presence?
   - *Recommendation*: Walk up from CWD to find `.replica/config.json`, fallback to git root

2. **Workspace-project linkage**: Should workspace DB track which project it came from?
   - *Recommendation*: Not initially. GC uses project config only when invoked from project dir.

3. **Config validation**: Strict (error on unknown keys) or lenient (ignore unknown)?
   - *Recommendation*: Lenient with warnings for forward compatibility

4. **Multiple projects on same REPLICA_ROOT**: How does GC handle different project configs?
   - *Recommendation*: GC uses the project config of the directory it's invoked from. Global env vars still win.

---

## Validation Plan

### Manual Testing
```bash
# 1. Init a new project
cd my-project
replica init

# 2. Verify config created
cat .replica/config.json
cat .replica/config.local.json
grep config.local.json .gitignore

# 3. Use config defaults
replica new  # uses default_ttl, default_name_template, post_clone_hooks
replica base build  # uses repo, setup_cmd from config

# 4. Override with flags
replica new --ttl 1d  # overrides config
replica base build --repo other --setup "other"  # overrides config

# 5. Project-specific agent config
replica setup  # installs project skills, appends project templates

# 6. Project-aware GC
replica gc  # uses project high/low water marks
```

### Automated Tests
- `tests/config.rs` - Config loading, merging, precedence
- `tests/init.rs` - Init command creates valid config
- `tests/cli_integration.rs` - `new`, `base build`, `gc`, `setup` respect config

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Config file syntax errors | Commands fail confusingly | Clear error messages with line numbers, `replica init --yes` for valid defaults |
| Conflicting global + project config | Unexpected behavior | Document precedence clearly, `--dry-run` shows effective config |
| Gitignore not updated | Local config committed | `init` auto-adds to `.gitignore`, warn if already tracked |
| Project root detection ambiguity | Wrong config used | Explicit `--project-root` flag on all commands as escape hatch |

---

## Future Extensions (Out of Scope)

- `replica config get/set` for programmatic config access
- Config inheritance (monorepo: root config + package config)
- Remote config via HTTP (for CI/CD)
- Config schema validation with JSON Schema
- `replica doctor` to validate project setup

---

## Task Breakdown

| # | Task | Dependencies |
|---|------|--------------|
| 1 | Create `src/config.rs` with schema, loading, merging | - |
| 2 | Add `Init` command to `main.rs` CLI | 1 |
| 3 | Implement `src/init.rs` with interactive prompts | 1 |
| 4 | Wire project config into `replica new` | 1 |
| 5 | Wire project config into `replica base build` | 1 |
| 6 | Wire project config into `replica setup` (agents, skills) | 1 |
| 7 | Wire project config into `replica gc` (water marks) | 1 |
| 8 | Add `config` help topic and update CLI help | 2, 3 |
| 9 | Write `docs/CONFIG.md` and update `docs/CLI.md` | 8 |
| 10 | Add unit and integration tests | 1-7 |
| 11 | Update `.gitignore` template in `init` | 3 |

---

## Estimated Effort
- Core config + init: ~3-4 focused sessions
- Command wiring: ~2-3 sessions
- Documentation + tests: ~2 sessions
- **Total: ~7-9 sessions** (implementation-ready plan)