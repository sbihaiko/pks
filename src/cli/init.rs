use std::path::{Path, PathBuf};
use std::process::Command;

use crate::git::BareCommit;

/// Error type for `pks init`.
#[derive(Debug)]
pub enum InitError {
    NotAGitRepo,
    Io(std::io::Error),
    AlreadyInitialized,
    Git(git2::Error),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::NotAGitRepo => write!(f, "diretório atual não é um repositório Git. Execute `git init` antes de usar `pks init`."),
            InitError::Io(e) => write!(f, "erro de I/O: {e}"),
            InitError::AlreadyInitialized => write!(f, "PKS já inicializado aqui. Use --force para sobrescrever."),
            InitError::Git(e) => write!(f, "erro Git: {e}"),
        }
    }
}

impl From<std::io::Error> for InitError {
    fn from(e: std::io::Error) -> Self { InitError::Io(e) }
}

impl From<git2::Error> for InitError {
    fn from(e: git2::Error) -> Self { InitError::Git(e) }
}

pub struct InitCommand {
    pub project_path: PathBuf,
    pub force: bool,
}

impl InitCommand {
    pub fn new(project_path: PathBuf, force: bool) -> Self {
        Self { project_path, force }
    }

    /// Runs all onboarding steps — completes in < 30s for repos up to 500 .md files.
    pub fn run(&self) -> Result<(), InitError> {
        let git_root = detect_git_root(&self.project_path)?;
        println!("✓ Git root detectado: {}", git_root.display());

        let config_path = git_root.join(".pks").join("config.toml");
        if config_path.exists() && !self.force {
            return Err(InitError::AlreadyInitialized);
        }
        let git_common_dir = detect_git_common_dir(&git_root)?;
        let project_name = git_root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());
        generate_config(&config_path, &project_name, &git_common_dir)?;
        println!("✓ Config gerada: {}", config_path.display());

        let bc = BareCommit::new(&git_root);
        bc.ensure_branch()?;
        seed_vault_skeleton(&bc)?;
        println!("✓ Branch criada: pks-knowledge (com estrutura Obsidian)");

        setup_worktree(&git_root);

        let repo_id = git_common_dir.to_string_lossy().into_owned();
        register_with_daemon(&repo_id);
        println!("✓ Daemon registrado: {project_name} (RepoId: {repo_id})");

        crate::vault_init::install_post_commit_hook(&git_root)
            .unwrap_or_else(|e| eprintln!("⚠ Aviso: hook install failed: {e}"));
        if let Err(e) = add_pks_to_exclude(&git_root) {
            eprintln!("⚠ Aviso: could not update .git/info/exclude: {e}");
        }
        if let Err(e) = hide_worktree_from_vscode(&git_root) {
            eprintln!("⚠ Aviso: could not update .vscode/settings.json: {e}");
        }
        println!("PKS ativo. Buscar: pks search \"<sua consulta>\"");
        Ok(())
    }
}

/// Returns true if `.pks/config.toml` exists at the given path.
pub fn is_initialized(path: &Path) -> bool {
    path.join(".pks").join("config.toml").exists()
}

pub(crate) fn detect_git_root(start: &Path) -> Result<PathBuf, InitError> {
    let output = Command::new("git")
        .args(["-C", start.to_str().unwrap_or("."), "rev-parse", "--show-toplevel"])
        .output()
        .map_err(InitError::Io)?;
    if !output.status.success() {
        return Err(InitError::NotAGitRepo);
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

fn detect_git_common_dir(git_root: &Path) -> Result<PathBuf, InitError> {
    use crate::git::RepoIdentity;
    RepoIdentity::from_path(git_root)
        .map(|id| id.git_common_dir)
        .map_err(InitError::Git)
}

pub(crate) fn generate_config(config_path: &Path, name: &str, git_common_dir: &Path) -> Result<(), InitError> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let common = git_common_dir.to_string_lossy();
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let content = format!(
        "# Gerado automaticamente por `pks init` em {now}\n\n\
         [project]\nname = \"{name}\"\ngit_common_dir = \"{common}\"\n\n\
         [indexing]\nenabled = true\nwatch_paths = [\".\"]\n\
         ignore_patterns = [\"target/\", \"node_modules/\", \".git/\", \"dist/\", \"build/\", \"*.lock\"]\n\n\
         [journal]\nshadow_journaling = false\nmin_words_per_entry = 50\n\n\
         [embedding]\nprovider = \"none\"\n"
    );
    let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(config_path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn seed_vault_skeleton(bc: &BareCommit) -> Result<(), InitError> {
    use crate::vault_init::{OBSIDIAN_APP_JSON, OBSIDIAN_WORKSPACE_JSON};
    let files: Vec<(&str, &[u8])> = vec![
        ("features/.gitkeep", b""),
        ("decisions/.gitkeep", b""),
        ("journals/.gitkeep", b""),
        (".obsidian/app.json", OBSIDIAN_APP_JSON.as_bytes()),
        (".obsidian/workspace.json", OBSIDIAN_WORKSPACE_JSON.as_bytes()),
    ];
    bc.write_files_batch(&files, "chore(pks): seed Obsidian vault structure")?;
    Ok(())
}

fn setup_worktree(git_root: &Path) {
    use crate::git_branch::{create_pks_branch_and_worktree, worktree_exists};
    if let Err(e) = create_pks_branch_and_worktree(git_root) {
        eprintln!("⚠ Aviso: worktree setup: {e}");
    } else {
        println!("✓ Worktree: prometheus/ → pks-knowledge");
    }
    // Ensure worktree files are checked out (handles pre-existing empty worktrees)
    if worktree_exists(git_root) {
        let prometheus = git_root.join("prometheus");
        let _ = Command::new("git")
            .current_dir(&prometheus)
            .args(["checkout", "pks-knowledge", "--", "."])
            .output();
    }
}

fn register_with_daemon(repo_id: &str) {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("pks"));
    let output = Command::new(exe).args(["refresh"]).output();
    if let Ok(o) = output {
        if !o.status.success() {
            eprintln!("⚠ Daemon offline — {repo_id} será registrado na próxima inicialização.");
        }
    }
}

fn add_pks_to_exclude(git_root: &Path) -> Result<(), std::io::Error> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    let exclude = git_root.join(".git").join("info").join("exclude");
    if let Some(p) = exclude.parent() { fs::create_dir_all(p)?; }
    let contents = fs::read_to_string(&exclude).unwrap_or_default();
    let mut f = OpenOptions::new().create(true).append(true).open(&exclude)?;
    if !contents.contains(".pks/") {
        writeln!(f, ".pks/")?;
    }
    if !contents.contains("prometheus/") {
        writeln!(f, "prometheus/")?;
    }
    Ok(())
}

fn hide_worktree_from_vscode(git_root: &Path) -> Result<(), std::io::Error> {
    use std::fs;
    let vscode_dir = git_root.join(".vscode");
    fs::create_dir_all(&vscode_dir)?;
    let settings_path = vscode_dir.join("settings.json");
    let mut obj: serde_json::Map<String, serde_json::Value> = if settings_path.exists() {
        let raw = fs::read_to_string(&settings_path)?;
        serde_json::from_str(&raw).unwrap_or_default()
    } else {
        serde_json::Map::new()
    };
    let key = "git.ignoredRepositories";
    let repos = obj.entry(key.to_string())
        .or_insert_with(|| serde_json::Value::Array(vec![]));
    if let serde_json::Value::Array(arr) = repos {
        let entry = serde_json::Value::String("prometheus".to_string());
        if !arr.contains(&entry) {
            arr.push(entry);
        }
    }
    fs::write(&settings_path, serde_json::to_string_pretty(&obj).unwrap())?;
    Ok(())
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;
