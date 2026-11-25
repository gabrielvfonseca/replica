# Project Configuration

Replica supports per-project configuration via a `.replica/` directory at the root of your git repository. This allows teams to share workspace defaults, base build settings, agent configurations, and GC parameters while allowing machine-specific overrides.

## Quick Start

```bash
cd your-project
replica init
```

This creates:
- `.replica/settings.json` - Team-shared config (track in git)
- `.replica/config.local.json` - Machine-specific overrides (gitignored)
- `.replica/templates/` - Project-specific agent templates (optional)
- `.replica/skills/` - Project-specific agent skills (optional)

And adds `.replica/config.local.json` to your `.gitignore`.

## Directory Structure

```
<project-root>/
├── .replica/
│   ├── settings.json          # Tracked in git (team defaults)
│   ├── config.local.json      # Gitignored (machine-specific overrides)
│   ├── skills/                # Project-specific agent skills (optional)
│   │   └── <agent>/SKILL.md
│   └── templates/             # Project-specific config templates (optional)
│       └── <agent>.md
└── .gitignore                 # Add: .replica/config.local.json
```

## Configuration Schema

### settings.json (tracked)

```json
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
    "ttl_override": null
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

### config.local.json (gitignored)

Override any setting for your local machine:

```json
{
  "base": { "repo": "https://github.com/myfork/repo.git" },
  "workspace": { "default_ttl": "4h" },
  "gc": { "high_water_mark": 70.0 }
}
```

Only include the fields you want to override. Missing fields fall back to `settings.json` or built-in defaults.

## Field Reference

### base
| Field | Type | Description |
|-------|------|-------------|
| `repo` | string | Git repository URL or local path. Used by `replica base build`. |
| `setup_cmd` | string | Command to run inside the clone to install dependencies (e.g., `"npm ci"`). |
| `default_branch` | string | Default branch for `--ref` flag (default: `"main"`). |

### workspace
| Field | Type | Description |
|-------|------|-------------|
| `default_ttl` | string | Default TTL for new workspaces: `"30m"`, `"2h"`, or `"1d"` (default: `"2h"`). |
| `default_name_template` | string | Template for `--name` (default: `"task-{{timestamp}}"`). |
| `post_clone_hooks` | string[] | Shell commands to run after cloning, joined with `&&`. |
| `require_fresh_default` | bool | Default for `--require-fresh` flag (default: `false`). |

### gc
| Field | Type | Description |
|-------|------|-------------|
| `high_water_mark` | number | Start evicting LRU workspaces above this volume usage % (default: `80.0`). |
| `low_water_mark` | number | Stop evicting once usage falls below this % (default: `60.0`). |
| `ttl_override` | string | Override TTL for GC expiry calculation (optional). |

### agents
Map of agent name to template configuration:
| Field | Type | Description |
|-------|------|-------------|
| `append` | string | Path to template file to append to agent's config. |
| `replace` | string | Path to template file to replace agent's config (requires `--force`). |

Supported agents: `claude`, `cursor`, `copilot`, `zed`, `aider`.

### skills
| Field | Type | Description |
|-------|------|-------------|
| `enabled` | bool | Enable project-specific skills (default: `true`). |
| `custom_skills_dir` | string | Directory containing project skills (default: `".replica/skills"`). |

## Precedence

Values are resolved in this order (highest wins):

1. **CLI flags** (e.g., `--ttl 1d`, `--repo <url>`)
2. **`.replica/config.local.json`** (machine-specific)
3. **`.replica/settings.json`** (project defaults)
4. **Built-in defaults**

## Commands Using Project Config

| Command | Config Fields Used |
|---------|-------------------|
| `replica new` | `workspace.default_ttl`, `workspace.default_name_template`, `workspace.post_clone_hooks`, `workspace.require_fresh_default`, `base.default_branch` |
| `replica base build` | `base.repo`, `base.setup_cmd` |
| `replica gc` | `gc.high_water_mark`, `gc.low_water_mark` |
| `replica setup` | `agents.*`, `skills.*` |

## Templates

Place agent-specific templates in `.replica/templates/`:
- `claude.md` - Appended to `~/.claude/CLAUDE.md`
- `cursorrules` - Replaces `.cursorrules`
- `copilot-instructions.md` - Appended to Copilot config
- `zed-settings.json` - Replaces Zed settings
- `aider.conf.yml` - Appended to Aider config

Reference them in `settings.json` under `agents.<name>.append` or `.replace`.

Example:
```json
"agents": {
  "claude": { "append": ".replica/templates/claude.md" },
  "cursor": { "replace": ".replica/templates/cursorrules" }
}
```

## Skills

Place project-specific skills in `.replica/skills/<agent>/SKILL.md`. They will be installed alongside the global Replica skill when you run `replica setup`.

Example structure:
```
.replica/
└── skills/
    └── claude/
        └── SKILL.md
```

## init Command

```bash
replica init [--path <dir>] [--yes] [--force]
```

| Flag | Description |
|------|-------------|
| `--path` | Project root (default: current directory) |
| `--yes` | Non-interactive: use defaults, don't prompt |
| `--force` | Overwrite existing config |

Interactive mode prompts for:
- Base repo URL and setup command
- Default TTL and name template
- Post-clone hooks
- Agent templates to include
- Skills configuration

## Example Workflow

```bash
# 1. Initialize project config
cd my-project
replica init

# 2. Edit .replica/settings.json with your repo and setup command
# 3. Build the base
replica base build

# 4. Create workspaces (uses config defaults)
replica new                    # Uses default_ttl, default_name_template
replica new --name feature-x  # Overrides name

# 5. Configure agents
replica setup                  # Installs project templates and skills

# 6. GC uses project water marks
replica gc
```

## Notes

- The `.replica/` directory should be at the root of your git repository (where `.git` is).
- `replica init` detects the git root automatically if run from a subdirectory.
- Run `replica help config` for a detailed guide.
- Project config is optional - all commands work without it using CLI flags or built-in defaults.