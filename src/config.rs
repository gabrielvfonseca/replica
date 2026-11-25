use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub gc: GcConfig,
    #[serde(default)]
    pub agents: AgentsConfig,
    #[serde(default)]
    pub skills: SkillsConfig,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            base: BaseConfig::default(),
            workspace: WorkspaceConfig::default_with_values(),
            gc: GcConfig::default(),
            agents: AgentsConfig::default(),
            skills: SkillsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseConfig {
    pub repo: Option<String>,
    pub setup_cmd: Option<String>,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceConfig {
    pub default_ttl: Option<String>,
    pub default_name_template: Option<String>,
    #[serde(default)]
    pub post_clone_hooks: Vec<String>,
    pub require_fresh_default: Option<bool>,
}

impl WorkspaceConfig {
    pub fn default_with_values() -> Self {
        Self {
            default_ttl: Some("2h".to_string()),
            default_name_template: Some("task-{{timestamp}}".to_string()),
            post_clone_hooks: Vec::new(),
            require_fresh_default: Some(false),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GcConfig {
    pub high_water_mark: Option<f64>,
    pub low_water_mark: Option<f64>,
    pub ttl_override: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    #[serde(flatten)]
    pub agents: HashMap<String, AgentTemplateConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplateConfig {
    pub append: Option<String>,
    pub replace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub enabled: Option<bool>,
    pub custom_skills_dir: Option<String>,
}

impl Default for SkillsConfig {
    fn default() -> Self {
        Self {
            enabled: Some(true),
            custom_skills_dir: Some(".replica/skills".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub project_root: PathBuf,
    pub config: ProjectConfig,
    pub local_config: ProjectConfig,
}

impl ProjectConfig {
    pub fn merge(&mut self, other: ProjectConfig) {
        self.base.merge(other.base);
        self.workspace.merge(other.workspace);
        self.gc.merge(other.gc);
        self.agents.merge(other.agents);
        self.skills.merge(other.skills);
    }
}

impl BaseConfig {
    fn merge(&mut self, other: BaseConfig) {
        if other.repo.is_some() {
            self.repo = other.repo;
        }
        if other.setup_cmd.is_some() {
            self.setup_cmd = other.setup_cmd;
        }
        if other.default_branch.is_some() {
            self.default_branch = other.default_branch;
        }
    }
}

impl WorkspaceConfig {
    fn merge(&mut self, other: WorkspaceConfig) {
        if other.default_ttl.is_some() {
            self.default_ttl = other.default_ttl;
        }
        if other.default_name_template.is_some() {
            self.default_name_template = other.default_name_template;
        }
        if !other.post_clone_hooks.is_empty() {
            self.post_clone_hooks = other.post_clone_hooks;
        }
        if other.require_fresh_default.is_some() {
            self.require_fresh_default = other.require_fresh_default;
        }
    }
}

impl GcConfig {
    fn merge(&mut self, other: GcConfig) {
        if other.high_water_mark.is_some() {
            self.high_water_mark = other.high_water_mark;
        }
        if other.low_water_mark.is_some() {
            self.low_water_mark = other.low_water_mark;
        }
        if other.ttl_override.is_some() {
            self.ttl_override = other.ttl_override;
        }
    }
}

impl AgentsConfig {
    fn merge(&mut self, other: AgentsConfig) {
        for (k, v) in other.agents {
            self.agents.insert(k, v);
        }
    }
}

impl SkillsConfig {
    fn merge(&mut self, other: SkillsConfig) {
        if other.enabled.is_some() {
            self.enabled = other.enabled;
        }
        if other.custom_skills_dir.is_some() {
            self.custom_skills_dir = other.custom_skills_dir;
        }
    }
}

pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start;
    loop {
        let config_path = current.join(".replica").join("settings.json");
        if config_path.exists() {
            return Some(current.to_path_buf());
        }
        if current.join(".git").exists() {
            let config_path = current.join(".replica").join("settings.json");
            if config_path.exists() {
                return Some(current.to_path_buf());
            }
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

fn read_config_file(path: &Path) -> Result<ProjectConfig> {
    if !path.exists() {
        return Ok(ProjectConfig::default());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading config file: {}", path.display()))?;
    let config: ProjectConfig = serde_json::from_str(&content)
        .with_context(|| format!("parsing JSON config: {}", path.display()))?;
    Ok(config)
}

pub fn load_project_config(project_root: &Path) -> Result<ResolvedConfig> {
    let config_path = project_root.join(".replica").join("settings.json");
    let local_config_path = project_root.join(".replica").join("config.local.json");

    let mut config = read_config_file(&config_path)?;
    let local_config = if local_config_path.exists() {
        let lc = read_config_file(&local_config_path)?;
        config.merge(lc.clone());
        lc
    } else {
        ProjectConfig::default()
    };

    Ok(ResolvedConfig {
        project_root: project_root.to_path_buf(),
        config,
        local_config,
    })
}

pub fn get_effective_config(
    project_root: Option<&Path>,
    cli_overrides: &CliOverrides,
) -> ProjectConfig {
    let mut config = if let Some(root) = project_root {
        load_project_config(root).map(|r| r.config).unwrap_or_default()
    } else {
        ProjectConfig::default()
    };

    if let Some(ttl) = &cli_overrides.ttl {
        config.workspace.default_ttl = Some(ttl.clone());
    }
    if let Some(name) = &cli_overrides.name {
        config.workspace.default_name_template = Some(name.clone());
    }
    if let Some(post_clone) = &cli_overrides.post_clone {
        config.workspace.post_clone_hooks = vec![post_clone.clone()];
    }
    if cli_overrides.require_fresh {
        config.workspace.require_fresh_default = Some(true);
    }
    if let Some(repo) = &cli_overrides.repo {
        config.base.repo = Some(repo.clone());
    }
    if let Some(setup) = &cli_overrides.setup_cmd {
        config.base.setup_cmd = Some(setup.clone());
    }
    if let Some(ref_branch) = &cli_overrides.ref_branch {
        config.base.default_branch = Some(ref_branch.clone());
    }
    if let Some(high) = cli_overrides.high {
        config.gc.high_water_mark = Some(high);
    }
    if let Some(low) = cli_overrides.low {
        config.gc.low_water_mark = Some(low);
    }

    config
}

#[derive(Debug, Default)]
pub struct CliOverrides {
    pub ttl: Option<String>,
    pub name: Option<String>,
    pub post_clone: Option<String>,
    pub require_fresh: bool,
    pub repo: Option<String>,
    pub setup_cmd: Option<String>,
    pub ref_branch: Option<String>,
    pub high: Option<f64>,
    pub low: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_merge() {
        let mut base = ProjectConfig::default();
        base.base.repo = Some("https://github.com/base/repo.git".to_string());
        base.workspace.default_ttl = Some("2h".to_string());

        let mut override_config = ProjectConfig::default();
        override_config.base.setup_cmd = Some("npm ci".to_string());
        override_config.workspace.default_ttl = Some("4h".to_string());

        base.merge(override_config);

        assert_eq!(base.base.repo, Some("https://github.com/base/repo.git".to_string()));
        assert_eq!(base.base.setup_cmd, Some("npm ci".to_string()));
        assert_eq!(base.workspace.default_ttl, Some("4h".to_string()));
    }

    #[test]
    fn test_load_project_config() {
        let dir = tempdir().unwrap();
        let replica_dir = dir.path().join(".replica");
        fs::create_dir_all(&replica_dir).unwrap();

        let config_json = r#"{"base":{"repo":"https://github.com/test/repo.git"},"workspace":{"default_ttl":"1d"}}"#;
        fs::write(replica_dir.join("settings.json"), config_json).unwrap();

        let resolved = load_project_config(dir.path()).unwrap();
        assert_eq!(resolved.config.base.repo, Some("https://github.com/test/repo.git".to_string()));
        assert_eq!(resolved.config.workspace.default_ttl, Some("1d".to_string()));
    }

    #[test]
    fn test_load_with_local_override() {
        let dir = tempdir().unwrap();
        let replica_dir = dir.path().join(".replica");
        fs::create_dir_all(&replica_dir).unwrap();

        let config_json = r#"{"base":{"repo":"https://github.com/test/repo.git"},"workspace":{"default_ttl":"2h"}}"#;
        fs::write(replica_dir.join("settings.json"), config_json).unwrap();

        let local_json = r#"{"workspace":{"default_ttl":"4h"}}"#;
        fs::write(replica_dir.join("config.local.json"), local_json).unwrap();

        let resolved = load_project_config(dir.path()).unwrap();
        assert_eq!(resolved.config.base.repo, Some("https://github.com/test/repo.git".to_string()));
        assert_eq!(resolved.config.workspace.default_ttl, Some("4h".to_string()));
    }

    #[test]
    fn test_find_project_root() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("project");
        let replica_dir = project.join(".replica");
        fs::create_dir_all(&replica_dir).unwrap();
        fs::write(replica_dir.join("settings.json"), "{}").unwrap();

        let subdir = project.join("subdir");
        fs::create_dir_all(&subdir).unwrap();

        let found = find_project_root(&subdir);
        assert_eq!(found, Some(project));
    }

    #[test]
    fn test_find_project_root_not_found() {
        let dir = tempdir().unwrap();
        let found = find_project_root(dir.path());
        assert_eq!(found, None);
    }
}