/// Reflexive Global Memory writer — T4.7 (F2.6)
///
/// Writes AI session summaries to `prometheus/journals/YYYY-MM-DD.md`
/// in the pks-knowledge branch. Committed with source_commit_sha for
/// bidirectional traceability (code <-> context).
///
/// Trigger: explicit MCP tool call `pks_session_summary(session_context)`.
use std::path::Path;
use tracing::info;

#[derive(Debug)]
pub struct SessionSummary {
    pub date: String,
    pub session_sha: String,
    pub topics: Vec<String>,
    pub repos_touched: Vec<String>,
    pub decisions: Vec<String>,
    pub context: String,
    pub next_steps: Vec<String>,
}

impl SessionSummary {
    /// Render the session summary as a Markdown document with YAML frontmatter.
    pub fn render(&self) -> String {
        let topics = self.topics.join(", ");
        let repos = self.repos_touched.join(", ");

        let decisions = if self.decisions.is_empty() {
            "- (nenhuma decisao registrada)".to_string()
        } else {
            self.decisions.iter().map(|d| format!("- {d}")).collect::<Vec<_>>().join("\n")
        };

        let next_steps = if self.next_steps.is_empty() {
            "- (nenhum proximo passo registrado)".to_string()
        } else {
            self.next_steps.iter().map(|s| format!("- {s}")).collect::<Vec<_>>().join("\n")
        };

        format!(
            "---\n\
             session_sha: {sha}\n\
             date: {date}\n\
             topics: [{topics}]\n\
             repos_touched: [{repos}]\n\
             ---\n\n\
             # Memoria de Sessao — {date}\n\n\
             ## Decisoes\n\n\
             {decisions}\n\n\
             ## Contexto\n\n\
             {ctx}\n\n\
             ## Proximos Passos\n\n\
             {next_steps}\n",
            sha = self.session_sha,
            date = self.date,
            topics = topics,
            repos = repos,
            ctx = self.context,
            decisions = decisions,
            next_steps = next_steps,
        )
    }

    /// File path relative to the prometheus/ root (journals/YYYY-MM-DD.md).
    pub fn file_rel_path(&self) -> String {
        format!("journals/{}.md", self.date)
    }
}

/// Write a session summary to the prometheus/ worktree and commit it
/// to the `pks-knowledge` branch.
pub fn write_session_memory(
    prometheus_root: &Path,
    summary: &SessionSummary,
    source_commit_sha: &str,
) -> Result<(), String> {
    let content = summary.render();
    let file_rel = summary.file_rel_path();
    let message = format!("feat(pks-knowledge): session memory {}", summary.date);

    crate::git_branch::commit_to_pks_knowledge(
        prometheus_root,
        &file_rel,
        &content,
        source_commit_sha,
        &message,
    )
    .map_err(|e| e.to_string())?;

    info!(
        date = %summary.date,
        sha = %source_commit_sha,
        "session memory committed to pks-knowledge"
    );
    Ok(())
}

/// Pure rendering: produce summary Markdown without writing to disk.
/// Used by MCP tool `pks_session_summary`.
pub fn render_session_summary(summary: &SessionSummary) -> String {
    summary.render()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_summary() -> SessionSummary {
        SessionSummary {
            date: "2026-03-09".to_string(),
            session_sha: "abc123def456".to_string(),
            topics: vec!["PKS M4".to_string(), "Git Journaling".to_string()],
            repos_touched: vec!["WellzestaNotion".to_string()],
            decisions: vec!["Usar branch orfao para pks-knowledge".to_string()],
            context: "Implementacao do milestone M4 do PKS Daemon.".to_string(),
            next_steps: vec!["Iniciar Fase C (M5 Embeddings)".to_string()],
        }
    }

    #[test]
    fn render_includes_yaml_frontmatter() {
        let s = sample_summary();
        let rendered = s.render();
        assert!(rendered.starts_with("---\n"), "must start with frontmatter");
        assert!(rendered.contains("session_sha: abc123def456"));
        assert!(rendered.contains("date: 2026-03-09"));
        assert!(rendered.contains("topics: [PKS M4, Git Journaling]"));
    }

    #[test]
    fn render_includes_all_sections() {
        let s = sample_summary();
        let rendered = s.render();
        assert!(rendered.contains("## Decisoes"));
        assert!(rendered.contains("## Contexto"));
        assert!(rendered.contains("## Proximos Passos"));
        assert!(rendered.contains("Usar branch orfao para pks-knowledge"));
        assert!(rendered.contains("Iniciar Fase C (M5 Embeddings)"));
    }

    #[test]
    fn file_rel_path_uses_date() {
        let s = sample_summary();
        assert_eq!(s.file_rel_path(), "journals/2026-03-09.md");
    }

    #[test]
    fn render_with_empty_decisions_shows_placeholder() {
        let mut s = sample_summary();
        s.decisions.clear();
        s.next_steps.clear();
        let rendered = s.render();
        assert!(rendered.contains("(nenhuma decisao registrada)"));
        assert!(rendered.contains("(nenhum proximo passo registrado)"));
    }
}
