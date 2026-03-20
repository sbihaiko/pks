use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use chrono::Utc;

use crate::git::BareCommit;
use crate::hooks::journal_entry::{
    JournalConfig, JournalEntry, ToolEvent, redact_secrets, truncate_summary,
};

/// Passive shadow journal — accumulates tool events in memory (zero I/O)
/// and flushes to `pks-knowledge` branch at session end via BareCommit.
pub struct ShadowJournalHook {
    pub(crate) repo_path: PathBuf,
    pub(crate) session_id: String,
    pub(crate) started_at: chrono::DateTime<Utc>,
    pub(crate) entries: Vec<JournalEntry>,
    pub(crate) dropped_events: usize,
    pub(crate) config: JournalConfig,
}

impl ShadowJournalHook {
    pub fn new(repo_path: PathBuf, session_id: String) -> Self {
        Self {
            repo_path,
            session_id,
            started_at: Utc::now(),
            entries: Vec::new(),
            dropped_events: 0,
            config: JournalConfig::default(),
        }
    }

    /// Constructs a ShadowJournalHook from pre-loaded entries (for flush-session reading from JSONL).
    pub fn from_entries(
        repo_path: PathBuf,
        session_id: String,
        started_at: chrono::DateTime<Utc>,
        entries: Vec<JournalEntry>,
    ) -> Self {
        Self {
            repo_path,
            session_id,
            started_at,
            entries,
            dropped_events: 0,
            config: JournalConfig::default(),
        }
    }

    /// Records a tool event in memory — zero I/O.
    pub fn record_tool_event(&mut self, event: ToolEvent) {
        if !self.config.enabled {
            return;
        }
        if self.entries.len() >= self.config.max_entries {
            self.dropped_events += 1;
            return;
        }
        let summary =
            redact_secrets(truncate_summary(&event.input_summary, self.config.truncate_chars));
        self.entries.push(JournalEntry {
            timestamp: Utc::now(),
            tool_name: event.tool_name,
            tool_input_summary: summary,
            outcome: event.outcome,
            file_paths: event.file_paths,
            decision_note: event.decision_note,
        });
    }

    /// Flushes entries to `pks-knowledge` via BareCommit.
    /// Returns `Ok(())` if disabled, session too short, or on graceful error.
    pub fn flush_to_vault(&self, bare_commit: &BareCommit) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }
        let word_count: usize = self
            .entries
            .iter()
            .flat_map(|e| e.tool_input_summary.split_whitespace())
            .count();
        if word_count < self.config.min_words {
            return Ok(());
        }
        let content = self.render_journal_md();
        let date = self.started_at.format("%Y-%m-%d").to_string();
        let file_path = format!("journals/{}_{}.md", date, self.session_id);
        bare_commit
            .ensure_branch()
            .map_err(|e| format!("ensure_branch: {e}"))?;
        bare_commit
            .write_file(
                &file_path,
                content.as_bytes(),
                &format!("pks(journal): session {}", self.session_id),
            )
            .map_err(|e| format!("write_file: {e}"))?;
        Ok(())
    }

    pub(crate) fn render_journal_md(&self) -> String {
        let date = self.started_at.format("%Y-%m-%d").to_string();
        let started = self.started_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let duration_mins = Utc::now().signed_duration_since(self.started_at).num_minutes();
        let tools_summary = render_tools_summary(&self.entries);
        let mut md = format!("# Session Journal: {date} — {}\n\n", self.session_id);
        md.push_str(&format!("**Início:** {started}\n**Duração:** {duration_mins}min\n"));
        md.push_str(&format!("**Ferramentas usadas:** {tools_summary}\n\n"));
        render_decisions_section(&self.entries, &mut md);
        render_files_table(&self.entries, &mut md);
        render_events_table(&self.entries, self.dropped_events, &mut md);
        md
    }
}

fn render_tools_summary(entries: &[JournalEntry]) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for e in entries { *counts.entry(e.tool_name.as_str()).or_default() += 1; }
    let mut sorted: Vec<_> = counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.iter().map(|(n, c)| format!("{n} ({c})")).collect::<Vec<_>>().join(", ")
}

fn render_decisions_section(entries: &[JournalEntry], md: &mut String) {
    md.push_str("## Decisões\n\n");
    let decisions: Vec<String> = entries.iter().filter_map(|e| {
        e.decision_note.as_ref().map(|note| {
            if e.file_paths.is_empty() { format!("- {note}") }
            else { format!("- {note} ({})", e.file_paths.join(", ")) }
        })
    }).collect();
    if decisions.is_empty() {
        md.push_str("_Nenhuma decisão registrada nesta sessão._\n\n");
    } else {
        for d in &decisions { md.push_str(d); md.push('\n'); }
        md.push('\n');
    }
}

fn render_files_table(entries: &[JournalEntry], md: &mut String) {
    let mut seen: HashSet<String> = HashSet::new();
    let mut files: Vec<(&str, &str, &str)> = Vec::new();
    for e in entries {
        for fp in &e.file_paths {
            if seen.insert(fp.clone()) {
                files.push((fp, &e.tool_name, &e.tool_input_summary));
            }
        }
    }
    md.push_str("## Arquivos Modificados\n\n");
    if files.is_empty() {
        md.push_str("_Nenhum arquivo modificado nesta sessão._\n\n");
    } else {
        md.push_str("| Arquivo | Operação | Resumo |\n|---------|----------|--------|\n");
        for (fp, op, summ) in &files {
            md.push_str(&format!("| `{fp}` | {op} | {} |\n", truncate_summary(summ, 60)));
        }
        md.push('\n');
    }
}

fn render_events_table(entries: &[JournalEntry], dropped: usize, md: &mut String) {
    md.push_str("## Eventos Detalhados\n\n| Timestamp | Ferramenta | Outcome | Paths |\n|-----------|------------|---------|-------|\n");
    for e in entries {
        let ts = e.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let paths = if e.file_paths.is_empty() { "—".to_owned() } else { e.file_paths.join(", ") };
        md.push_str(&format!("| {ts} | {} | {} | {paths} |\n", e.tool_name, e.outcome));
    }
    if dropped > 0 {
        md.push_str(&format!("\n_{dropped} events dropped (max_entries reached)._\n"));
    }
}

#[cfg(test)]
#[path = "shadow_journal_tests.rs"]
mod tests;
