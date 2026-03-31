use std::fs;
use std::path::Path;

const PROMETHEUS_DIRS: &[&str] = &[
    "prometheus/features",
    "prometheus/decisions",
    "prometheus/journals",
];

const OBSIDIAN_APP_JSON: &str = r#"{
  "legacyEditor": false,
  "livePreview": true,
  "defaultViewMode": "source"
}
"#;

const OBSIDIAN_WORKSPACE_JSON: &str = r#"{
  "main": { "id": "main", "type": "split", "direction": "vertical", "items": [] },
  "left": { "id": "left", "type": "leaves", "value": [] },
  "right": { "id": "right", "type": "leaves", "value": [] }
}
"#;

#[derive(Debug)]
pub enum VaultInitError {
    NotAGitRepo,
    Io(std::io::Error),
}

impl From<std::io::Error> for VaultInitError {
    fn from(e: std::io::Error) -> Self { VaultInitError::Io(e) }
}

impl std::fmt::Display for VaultInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VaultInitError::NotAGitRepo => write!(f, "not a git repository"),
            VaultInitError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

pub struct VaultInitResult {
    pub dirs_created: Vec<String>,
    pub was_idempotent: bool,
}

pub fn init_vault(repo_root: &Path) -> Result<VaultInitResult, VaultInitError> {
    if !repo_root.join(".git").is_dir() {
        return Err(VaultInitError::NotAGitRepo);
    }

    let mut dirs_created = Vec::new();
    let mut was_idempotent = true;

    for dir in PROMETHEUS_DIRS {
        let full_path = repo_root.join(dir);
        if full_path.exists() {
            continue;
        }
        fs::create_dir_all(&full_path)?;
        fs::write(full_path.join(".gitkeep"), "")?;
        dirs_created.push(dir.to_string());
        was_idempotent = false;
    }

    let obsidian_dir = repo_root.join("prometheus/.obsidian");
    if !obsidian_dir.exists() {
        fs::create_dir_all(&obsidian_dir)?;
        was_idempotent = false;
    }
    let app_json = obsidian_dir.join("app.json");
    if !app_json.exists() {
        fs::write(&app_json, OBSIDIAN_APP_JSON)?;
    }
    let workspace_json = obsidian_dir.join("workspace.json");
    if !workspace_json.exists() {
        fs::write(&workspace_json, OBSIDIAN_WORKSPACE_JSON)?;
    }

    Ok(VaultInitResult { dirs_created, was_idempotent })
}

pub fn add_to_git_exclude(repo_root: &Path) -> std::io::Result<()> {
    let exclude_path = repo_root.join(".git/info/exclude");
    if let Ok(current) = fs::read_to_string(&exclude_path) {
        if current.contains("prometheus/") {
            return Ok(());
        }
    }
    let mut content = fs::read_to_string(&exclude_path).unwrap_or_default();
    if !content.ends_with('\n') && !content.is_empty() {
        content.push('\n');
    }
    content.push_str("prometheus/\n");
    fs::write(exclude_path, content)
}

const PKS_HOOK_BLOCK: &str = r#"
# --- PKS post-commit hook — hard timeout 100ms to never block git commit ---
SHA=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "unknown")

if command -v gtimeout >/dev/null 2>&1; then
    gtimeout 0.1s pks hook-post-commit . "$SHA" "$BRANCH" 2>/dev/null &
elif command -v timeout >/dev/null 2>&1; then
    timeout 0.1s pks hook-post-commit . "$SHA" "$BRANCH" 2>/dev/null &
else
    pks hook-post-commit . "$SHA" "$BRANCH" 2>/dev/null &
fi
"#;

pub fn install_post_commit_hook(repo_root: &Path) -> std::io::Result<()> {
    let hooks_dir = repo_root.join(".git/hooks");
    fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join("post-commit");

    let existing = fs::read_to_string(&hook_path).unwrap_or_default();
    if existing.contains("pks hook-post-commit") {
        return Ok(());
    }

    let mut content = if existing.is_empty() {
        "#!/bin/sh\n".to_string()
    } else {
        existing
    };
    content.push_str(PKS_HOOK_BLOCK);
    fs::write(&hook_path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_git_repo(dir: &TempDir) -> std::path::PathBuf {
        let repo = dir.path().to_path_buf();
        fs::create_dir_all(repo.join(".git/info")).unwrap();
        repo
    }

    #[test]
    fn init_vault_creates_all_prometheus_dirs() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_repo(&dir);

        let result = init_vault(&repo).unwrap();
        assert!(!result.was_idempotent);
        assert_eq!(result.dirs_created.len(), PROMETHEUS_DIRS.len());

        for d in PROMETHEUS_DIRS {
            assert!(repo.join(d).is_dir(), "{d} was not created");
        }
    }

    #[test]
    fn init_vault_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_repo(&dir);
        init_vault(&repo).unwrap();
        let second = init_vault(&repo).unwrap();
        assert!(second.was_idempotent);
        assert!(second.dirs_created.is_empty());
    }

    #[test]
    fn init_vault_fails_on_non_git_dir() {
        let dir = TempDir::new().unwrap();
        let result = init_vault(dir.path());
        assert!(matches!(result, Err(VaultInitError::NotAGitRepo)));
    }

    #[test]
    fn add_to_git_exclude_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_repo(&dir);
        add_to_git_exclude(&repo).unwrap();
        add_to_git_exclude(&repo).unwrap();

        let content = fs::read_to_string(repo.join(".git/info/exclude")).unwrap();
        assert_eq!(content.matches("prometheus/").count(), 1);
    }

    #[test]
    fn install_post_commit_hook_contains_timeout() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_repo(&dir);
        fs::create_dir_all(repo.join(".git/hooks")).unwrap();
        install_post_commit_hook(&repo).unwrap();
        let content = fs::read_to_string(repo.join(".git/hooks/post-commit")).unwrap();
        assert!(content.contains("gtimeout") || content.contains("timeout"));
        assert!(content.contains("pks hook-post-commit"));
    }

    #[test]
    fn install_post_commit_hook_is_idempotent() {
        let dir = TempDir::new().unwrap();
        let repo = make_git_repo(&dir);
        fs::create_dir_all(repo.join(".git/hooks")).unwrap();
        install_post_commit_hook(&repo).unwrap();
        let first = fs::read_to_string(repo.join(".git/hooks/post-commit")).unwrap();
        install_post_commit_hook(&repo).unwrap();
        let second = fs::read_to_string(repo.join(".git/hooks/post-commit")).unwrap();
        assert_eq!(first, second, "hook must be identical after second install");
    }

    #[test]
    fn init_vault_zettelkasten_structure_also_indexes() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().to_path_buf();
        fs::create_dir_all(repo.join(".git/info")).unwrap();
        fs::write(repo.join("20230101-idea.md"), "# Idea\n\nContent here.").unwrap();
        fs::write(repo.join("20230102-followup.md"), "# Followup\n\nMore content.").unwrap();

        let result = init_vault(&repo).unwrap();
        assert!(!result.was_idempotent);
        assert!(repo.join("20230101-idea.md").exists());
    }
}
