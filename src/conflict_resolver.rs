use std::path::Path;
use tracing::warn;

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictKind {
    ConcurrentEdit { file_path: String },
    MergeMarkers { file_path: String },
    CommitConflict { automation_sha: String, human_sha: String },
}

#[derive(Debug, Clone)]
pub struct ConflictResolution {
    pub kind: ConflictKind,
    pub strategy: ResolutionStrategy,
    pub winner: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionStrategy {
    HumanWins,
    AutomationWins,
    LastWriterWins,
}

pub struct ConflictResolver;

impl ConflictResolver {
    pub fn resolve(
        kind: ConflictKind,
        human_content: &str,
        automation_content: &str,
    ) -> ConflictResolution {
        let _ = automation_content;
        if has_merge_markers(human_content) {
            warn!(
                conflict = ?kind,
                strategy = "HumanWins",
                "merge markers detected — keeping human version, discarding automation"
            );
            return ConflictResolution {
                kind,
                strategy: ResolutionStrategy::HumanWins,
                winner: human_content.to_string(),
            };
        }

        warn!(
            conflict = ?kind,
            strategy = "HumanWins",
            "concurrent edit conflict — human version preserved"
        );
        ConflictResolution {
            kind,
            strategy: ResolutionStrategy::HumanWins,
            winner: human_content.to_string(),
        }
    }

    pub fn resolve_by_mtime(
        file_path: &Path,
        automation_content: &str,
    ) -> Option<ConflictResolution> {
        let metadata = std::fs::metadata(file_path).ok()?;
        let mtime = metadata.modified().ok()?;
        let age_secs = mtime
            .elapsed()
            .map(|d| d.as_secs())
            .unwrap_or(u64::MAX);

        if age_secs >= 2 {
            return None;
        }

        let human_content = std::fs::read_to_string(file_path).unwrap_or_default();
        let kind = ConflictKind::ConcurrentEdit {
            file_path: file_path.to_string_lossy().into_owned(),
        };
        Some(ConflictResolver::resolve(kind, &human_content, automation_content))
    }

    pub fn strip_conflict_markers(content: &str) -> String {
        let mut result = Vec::new();
        let mut in_ours = true;
        let mut in_conflict = false;

        for line in content.lines() {
            if line.starts_with("<<<<<<<") {
                in_conflict = true;
                in_ours = true;
                continue;
            }
            if line.starts_with("=======") && in_conflict {
                in_ours = false;
                continue;
            }
            if line.starts_with(">>>>>>>") && in_conflict {
                in_conflict = false;
                in_ours = true;
                continue;
            }
            if in_ours || !in_conflict {
                result.push(line);
            }
        }

        result.join("\n")
    }
}

fn has_merge_markers(content: &str) -> bool {
    content.contains("<<<<<<<") && content.contains("=======") && content.contains(">>>>>>>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_wins_by_default() {
        let kind = ConflictKind::ConcurrentEdit { file_path: "a.md".to_string() };
        let resolution = ConflictResolver::resolve(kind, "human content", "automation content");
        assert_eq!(resolution.strategy, ResolutionStrategy::HumanWins);
        assert_eq!(resolution.winner, "human content");
    }

    #[test]
    fn merge_markers_detected_and_resolved() {
        let content_with_markers = "# Note\n<<<<<<< HEAD\nhuman line\n=======\nautomation line\n>>>>>>> pks\n";
        let kind = ConflictKind::MergeMarkers { file_path: "b.md".to_string() };
        let resolution = ConflictResolver::resolve(kind, content_with_markers, "auto");
        assert_eq!(resolution.strategy, ResolutionStrategy::HumanWins);
    }

    #[test]
    fn strip_conflict_markers_keeps_ours_side() {
        let content = "before\n<<<<<<< HEAD\nours line\n=======\ntheirs line\n>>>>>>> pks\nafter";
        let stripped = ConflictResolver::strip_conflict_markers(content);
        assert!(stripped.contains("ours line"), "ours line should be kept");
        assert!(!stripped.contains("theirs line"), "theirs line should be removed");
        assert!(!stripped.contains("<<<<<<<"), "markers should be stripped");
    }

    #[test]
    fn no_conflict_markers_returns_content_unchanged_via_has_merge_markers() {
        assert!(!has_merge_markers("clean content without markers"));
        assert!(has_merge_markers("<<<<<<< HEAD\na\n=======\nb\n>>>>>>> pks"));
    }
}
