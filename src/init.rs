use anyhow::{Context, Result};
use crate::config::{ProjectConfig, BaseConfig, WorkspaceConfig, GcConfig, AgentsConfig, SkillsConfig, AgentTemplateConfig};
use serde_json;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_JSON: &str = r#"{
  "base": {
    "repo": "",
    "setup_cmd": "",
    "default_branch": "main"
  },
  "workspace": {
    "default_ttl": "2h",
    "default_name_template": "task-{{timestamp}}",
    "post_clone_hooks": [],
    "require_fresh_default": false
  },
  "gc": {
    "high_water_mark": 80.0,
    "low_water_mark": 60.0,
    "ttl_override": null
  },
  "agents": {},
  "skills": {
    "enabled": true,
    "custom_skills_dir": ".replica/skills"
  }
}"#;

const DEFAULT_LOCAL_CONFIG_JSON: &str = r#"{
  "base": {},
  "workspace": {},
  "gc": {},
  "agents": {},
  "skills": {}
}"#;

pub fn run_init(project_path: Option<&Path>, yes: bool, force: bool) -> Result<()> {
    let project_root = project_path
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    println!("Replica Project Init");
    println!("====================");
    println!("Project root: {}", project_root.display());
    println!();

    let replica_dir = project_root.join(".replica");
    let config_path = replica_dir.join("settings.json");
    let local_config_path = replica_dir.join("config.local.json");
    let gitignore_path = project_root.join(".gitignore");

    if config_path.exists() && !force {
        println!("Config already exists at: {}", config_path.display());
        println!("Use --force to overwrite.");
        return Ok(());
    }

    if !yes {
        println!("This will create:");
        println!("  {}/settings.json (tracked in git)", replica_dir.display());
        println!("  {}/config.local.json (gitignored)", replica_dir.display());
        println!("  Update .gitignore to ignore config.local.json");
        println!();
        print!("Continue? [Y/n] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if !input.is_empty() && input != "y" && input != "yes" {
            println!("Aborted.");
            return Ok(());
        }
        println!();
    }

    fs::create_dir_all(&replica_dir)
        .with_context(|| format!("creating directory: {}", replica_dir.display()))?;

    fs::write(&config_path, DEFAULT_CONFIG_JSON)
        .with_context(|| format!("writing config: {}", config_path.display()))?;

    fs::write(&local_config_path, DEFAULT_LOCAL_CONFIG_JSON)
        .with_context(|| format!("writing local config: {}", local_config_path.display()))?;

    update_gitignore(&gitignore_path, "config.local.json")?;

    println!("Created project configuration:");
    println!("  {}", config_path.display());
    println!("  {}", local_config_path.display());
    println!("  Updated .gitignore");
    println!();

    if !yes {
        println!("Next steps:");
        println!("  1. Edit .replica/settings.json to set your base repo and setup command");
        println!("  2. Run 'replica base build' to build the golden tree");
        println!("  3. Run 'replica new' to create your first workspace");
        println!("  4. Run 'replica setup' to configure your coding agents");
    }

    Ok(())
}

fn update_gitignore(gitignore_path: &Path, entry: &str) -> Result<()> {
    let mut content = if gitignore_path.exists() {
        fs::read_to_string(gitignore_path)?
    } else {
        String::new()
    };

    let entry_line = format!(".replica/{}", entry);
    if !content.lines().any(|line| line.trim() == entry_line) {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&entry_line);
        content.push('\n');
        fs::write(gitignore_path, content)
            .with_context(|| format!("writing .gitignore: {}", gitignore_path.display()))?;
    }

    Ok(())
}

pub fn create_project_config_interactive(_project_root: &Path) -> Result<ProjectConfig> {
    println!("Configuring project...");
    println!();

    let mut config = ProjectConfig::default();

    config.base = prompt_base_config()?;
    config.workspace = prompt_workspace_config()?;
    config.gc = prompt_gc_config()?;
    config.agents = prompt_agents_config()?;
    config.skills = prompt_skills_config()?;

    Ok(config)
}

fn prompt_base_config() -> Result<BaseConfig> {
    println!("Base Configuration");
    println!("------------------");

    let repo = prompt_with_default("Git repo URL or local path", "")?;
    let setup_cmd = prompt_with_default("Setup command (e.g. 'npm ci', 'cargo fetch')", "")?;
    let default_branch = prompt_with_default("Default branch", "main")?;

    Ok(BaseConfig {
        repo: if repo.is_empty() { None } else { Some(repo) },
        setup_cmd: if setup_cmd.is_empty() { None } else { Some(setup_cmd) },
        default_branch: if default_branch.is_empty() { None } else { Some(default_branch) },
    })
}

fn prompt_workspace_config() -> Result<WorkspaceConfig> {
    println!();
    println!("Workspace Defaults");
    println!("------------------");

    let default_ttl = prompt_with_default("Default TTL (30m, 2h, 1d)", "2h")?;
    let default_name_template = prompt_with_default("Default name template", "task-{{timestamp}}")?;

    println!("Post-clone hooks (one per line, empty line to finish):");
    let mut post_clone_hooks = Vec::new();
    loop {
        print!("  Hook: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let hook = input.trim();
        if hook.is_empty() {
            break;
        }
        post_clone_hooks.push(hook.to_string());
    }

    let require_fresh = prompt_yes_no("Require fresh base by default?", false)?;

    Ok(WorkspaceConfig {
        default_ttl: Some(default_ttl),
        default_name_template: Some(default_name_template),
        post_clone_hooks,
        require_fresh_default: Some(require_fresh),
    })
}

fn prompt_gc_config() -> Result<GcConfig> {
    println!();
    println!("GC Configuration");
    println!("----------------");

    let high = prompt_with_default("High water mark (%)", "80")?.parse::<f64>().unwrap_or(80.0);
    let low = prompt_with_default("Low water mark (%)", "60")?.parse::<f64>().unwrap_or(60.0);

    Ok(GcConfig {
        high_water_mark: Some(high),
        low_water_mark: Some(low),
        ttl_override: None,
    })
}

fn prompt_agents_config() -> Result<AgentsConfig> {
    println!();
    println!("Agent Templates (optional)");
    println!("--------------------------");
    println!("Project-specific templates to append/replace in agent configs.");
    println!("Leave empty to skip.");

    let mut agents = std::collections::HashMap::new();

    let agents_list = ["claude", "cursor", "copilot", "zed", "aider"];
    for agent in agents_list {
        print!("  {} template (append/replace/skip): ", agent);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        if choice == "append" || choice == "a" {
            let path = format!(".replica/templates/{}.md", agent);
            agents.insert(agent.to_string(), AgentTemplateConfig {
                append: Some(path),
                replace: None,
            });
        } else if choice == "replace" || choice == "r" {
            let path = if agent == "cursor" {
                ".replica/templates/cursorrules".to_string()
            } else {
                format!(".replica/templates/{}.md", agent)
            };
            agents.insert(agent.to_string(), AgentTemplateConfig {
                append: None,
                replace: Some(path),
            });
        }
    }

    Ok(AgentsConfig { agents })
}

fn prompt_skills_config() -> Result<SkillsConfig> {
    println!();
    println!("Skills Configuration");
    println!("--------------------");

    let enabled = prompt_yes_no("Enable project-specific skills?", true)?;

    Ok(SkillsConfig {
        enabled: Some(enabled),
        custom_skills_dir: Some(".replica/skills".to_string()),
    })
}

fn prompt_with_default(prompt: &str, default: &str) -> Result<String> {
    if !default.is_empty() {
        print!("{} [{}]: ", prompt, default);
    } else {
        print!("{}: ", prompt);
    }
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    if input.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(input.to_string())
    }
}

fn prompt_yes_no(prompt: &str, default: bool) -> Result<bool> {
    let default_str = if default { "Y/n" } else { "y/N" };
    print!("{} [{}]: ", prompt, default_str);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();
    if input.is_empty() {
        Ok(default)
    } else {
        Ok(input == "y" || input == "yes")
    }
}

pub fn write_project_config(project_root: &Path, config: &ProjectConfig) -> Result<()> {
    let replica_dir = project_root.join(".replica");
    fs::create_dir_all(&replica_dir)?;

    let config_path = replica_dir.join("settings.json");
    let json = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, json)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_write_project_config() {
        let dir = tempdir().unwrap();
        let config = ProjectConfig::default();
        write_project_config(dir.path(), &config).unwrap();

        let config_path = dir.path().join(".replica").join("settings.json");
        assert!(config_path.exists());
        let content = fs::read_to_string(config_path).unwrap();
        let parsed: ProjectConfig = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.workspace.default_ttl, Some("2h".to_string()));
    }

    #[test]
    fn test_update_gitignore() {
        let dir = tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");

        update_gitignore(&gitignore, "config.local.json").unwrap();
        let content = fs::read_to_string(&gitignore).unwrap();
        assert!(content.contains(".replica/config.local.json"));

        // Second call should not duplicate
        update_gitignore(&gitignore, "config.local.json").unwrap();
        let content = fs::read_to_string(&gitignore).unwrap();
        let count = content.matches(".replica/config.local.json").count();
        assert_eq!(count, 1);
    }
}