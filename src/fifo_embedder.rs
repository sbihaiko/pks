use std::collections::VecDeque;
use std::io::{BufRead, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub enum OllamaState {
    Healthy,
    ModelMissing,
    TemporarilyOffline,
    ProlongedOffline,
    ReturnFromOffline,
}

#[derive(Serialize, Deserialize)]
struct DebtRecord {
    chunk_hash: String,
    vector: Vec<f32>,
}

pub struct FifoEmbedder {
    pub backlog: VecDeque<(String, Vec<f32>)>,
    pub state: OllamaState,
    pub debt_path: PathBuf,
    pub max_vectors: usize,
    pub hibernate_days: u64,
}

impl FifoEmbedder {
    pub fn from_env() -> Self {
        let max_vectors = std::env::var("PKS_MAX_VECTORS")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(500_000usize);
        let hibernate_days = std::env::var("PKS_HIBERNATE_DAYS")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(7u64);
        Self {
            backlog: VecDeque::new(),
            state: OllamaState::Healthy,
            debt_path: resolve_debt_path(),
            max_vectors,
            hibernate_days,
        }
    }

    pub fn enqueue_chunk(&mut self, chunk_hash: String, embedding: Vec<f32>) {
        self.backlog.push_back((chunk_hash, embedding));
    }

    pub fn backlog_depth(&self) -> usize {
        self.backlog.len()
    }

    pub fn serialize_overflow_to_debt(&mut self) {
        let parent = match self.debt_path.parent() {
            Some(p) => p.to_path_buf(),
            None => return,
        };
        if std::fs::create_dir_all(&parent).is_err() {
            return;
        }
        let file = match std::fs::OpenOptions::new()
            .create(true).append(true).open(&self.debt_path)
        {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut writer = std::io::BufWriter::new(file);
        while let Some((chunk_hash, vector)) = self.backlog.pop_front() {
            let record = DebtRecord { chunk_hash, vector };
            if let Ok(line) = serde_json::to_string(&record) {
                let _ = writeln!(writer, "{line}");
            }
        }
    }

    pub fn drain_debt_file(&mut self) {
        if !self.debt_path.exists() {
            return;
        }
        let file = match std::fs::File::open(&self.debt_path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let reader = std::io::BufReader::new(file);
        for line_result in reader.lines() {
            let line = match line_result { Ok(l) => l, Err(_) => continue };
            let record: DebtRecord = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(_) => continue,
            };
            self.backlog.push_back((record.chunk_hash, record.vector));
        }
        let _ = std::fs::remove_file(&self.debt_path);
    }
}

fn resolve_debt_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".pks").join("embedding_debt.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn embedder_in_dir(dir: &TempDir) -> FifoEmbedder {
        FifoEmbedder {
            backlog: VecDeque::new(),
            state: OllamaState::Healthy,
            debt_path: dir.path().join("embedding_debt.jsonl"),
            max_vectors: 500_000,
            hibernate_days: 7,
        }
    }

    #[test]
    fn from_env_creates_with_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PKS_MAX_VECTORS");
        std::env::remove_var("PKS_HIBERNATE_DAYS");
        let embedder = FifoEmbedder::from_env();
        assert_eq!(embedder.max_vectors, 500_000);
        assert_eq!(embedder.hibernate_days, 7);
        assert_eq!(embedder.state, OllamaState::Healthy);
    }

    #[test]
    fn from_env_reads_custom_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PKS_MAX_VECTORS", "1000");
        std::env::set_var("PKS_HIBERNATE_DAYS", "3");
        let embedder = FifoEmbedder::from_env();
        std::env::remove_var("PKS_MAX_VECTORS");
        std::env::remove_var("PKS_HIBERNATE_DAYS");
        assert_eq!(embedder.max_vectors, 1000);
        assert_eq!(embedder.hibernate_days, 3);
    }

    #[test]
    fn enqueue_chunk_increments_backlog_depth() {
        let dir = TempDir::new().unwrap();
        let mut embedder = embedder_in_dir(&dir);
        assert_eq!(embedder.backlog_depth(), 0);
        embedder.enqueue_chunk("hash-a".to_string(), vec![0.1, 0.2]);
        assert_eq!(embedder.backlog_depth(), 1);
        embedder.enqueue_chunk("hash-b".to_string(), vec![0.3]);
        assert_eq!(embedder.backlog_depth(), 2);
    }

    #[test]
    fn serialize_overflow_creates_file_and_empties_backlog() {
        let dir = TempDir::new().unwrap();
        let mut embedder = embedder_in_dir(&dir);
        embedder.enqueue_chunk("h1".to_string(), vec![1.0]);
        embedder.enqueue_chunk("h2".to_string(), vec![2.0]);
        embedder.serialize_overflow_to_debt();
        assert!(embedder.debt_path.exists());
        assert_eq!(embedder.backlog_depth(), 0);
    }

    #[test]
    fn drain_debt_file_reingests_and_removes_file() {
        let dir = TempDir::new().unwrap();
        let mut embedder = embedder_in_dir(&dir);
        embedder.enqueue_chunk("hx".to_string(), vec![9.0]);
        embedder.enqueue_chunk("hy".to_string(), vec![5.0]);
        embedder.serialize_overflow_to_debt();
        embedder.drain_debt_file();
        assert_eq!(embedder.backlog_depth(), 2);
        assert!(!embedder.debt_path.exists());
    }

    #[test]
    fn drain_debt_file_absent_file_is_noop() {
        let dir = TempDir::new().unwrap();
        let mut embedder = embedder_in_dir(&dir);
        embedder.drain_debt_file();
        assert_eq!(embedder.backlog_depth(), 0);
    }

    #[test]
    fn ollama_state_has_five_distinct_variants() {
        assert_ne!(OllamaState::Healthy, OllamaState::ModelMissing);
        assert_ne!(OllamaState::TemporarilyOffline, OllamaState::ProlongedOffline);
        assert_ne!(OllamaState::ProlongedOffline, OllamaState::ReturnFromOffline);
    }
}
