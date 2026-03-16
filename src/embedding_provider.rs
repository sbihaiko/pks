use serde::Deserialize;

#[derive(Debug)]
pub enum EmbeddingError {
    NetworkUnavailable(String),
    ModelNotFound(String),
    MalformedResponse(String),
    HttpRequestFailed(String),
}

impl std::fmt::Display for EmbeddingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbeddingError::NetworkUnavailable(msg) => write!(formatter, "network unavailable: {msg}"),
            EmbeddingError::ModelNotFound(msg) => write!(formatter, "model not found: {msg}"),
            EmbeddingError::MalformedResponse(msg) => write!(formatter, "malformed response: {msg}"),
            EmbeddingError::HttpRequestFailed(msg) => write!(formatter, "http request failed: {msg}"),
        }
    }
}

pub trait EmbeddingProvider {
    fn embed_text(
        &self,
        text: &str,
    ) -> impl std::future::Future<Output = Result<Vec<f32>, EmbeddingError>> + Send;
}

#[derive(Debug, Clone)]
pub struct OllamaProvider {
    pub base_url: String,
    pub model: String,
    pub throttle_ms: u64,
}

fn read_base_url() -> String {
    std::env::var("OLLAMA_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:11434".to_string())
}

fn read_model() -> String {
    std::env::var("OLLAMA_EMBED_MODEL")
        .unwrap_or_else(|_| "nomic-embed-text".to_string())
}

fn read_throttle_ms() -> u64 {
    std::env::var("PKS_THROTTLE_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200u64)
}

impl OllamaProvider {
    pub fn from_env() -> Self {
        Self {
            base_url: read_base_url(),
            model: read_model(),
            throttle_ms: read_throttle_ms(),
        }
    }
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

async fn post_to_ollama(
    endpoint: &str,
    body: &serde_json::Value,
) -> Result<reqwest::Response, EmbeddingError> {
    reqwest::Client::new()
        .post(endpoint)
        .json(body)
        .send()
        .await
        .map_err(|err| EmbeddingError::NetworkUnavailable(err.to_string()))
}

async fn decode_embedding_response(
    response: reqwest::Response,
    model: &str,
) -> Result<Vec<f32>, EmbeddingError> {
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(EmbeddingError::ModelNotFound(model.to_string()));
    }
    if !response.status().is_success() {
        return Err(EmbeddingError::HttpRequestFailed(response.status().to_string()));
    }
    response
        .json::<OllamaEmbeddingResponse>()
        .await
        .map(|r| r.embedding)
        .map_err(|err| EmbeddingError::MalformedResponse(err.to_string()))
}

impl EmbeddingProvider for OllamaProvider {
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let endpoint = format!("{}/api/embeddings", self.base_url);
        let body = serde_json::json!({"model": self.model, "prompt": text});
        let response = post_to_ollama(&endpoint, &body).await?;
        decode_embedding_response(response, &self.model).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn from_env_uses_defaults_when_vars_absent() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("OLLAMA_BASE_URL");
        std::env::remove_var("OLLAMA_EMBED_MODEL");
        std::env::remove_var("PKS_THROTTLE_MS");

        let provider = OllamaProvider::from_env();

        assert_eq!(provider.base_url, "http://localhost:11434");
        assert_eq!(provider.model, "nomic-embed-text");
        assert_eq!(provider.throttle_ms, 200);
    }

    #[test]
    fn from_env_reads_custom_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("OLLAMA_BASE_URL", "http://custom-host:9999");
        std::env::set_var("OLLAMA_EMBED_MODEL", "all-minilm");
        std::env::set_var("PKS_THROTTLE_MS", "500");

        let provider = OllamaProvider::from_env();

        std::env::remove_var("OLLAMA_BASE_URL");
        std::env::remove_var("OLLAMA_EMBED_MODEL");
        std::env::remove_var("PKS_THROTTLE_MS");

        assert_eq!(provider.base_url, "http://custom-host:9999");
        assert_eq!(provider.model, "all-minilm");
        assert_eq!(provider.throttle_ms, 500);
    }

    #[test]
    fn embedding_error_display_is_descriptive() {
        let err = EmbeddingError::NetworkUnavailable("timeout".to_string());
        assert!(err.to_string().contains("network unavailable"));

        let err = EmbeddingError::ModelNotFound("nomic".to_string());
        assert!(err.to_string().contains("model not found"));

        let err = EmbeddingError::MalformedResponse("bad json".to_string());
        assert!(err.to_string().contains("malformed response"));

        let err = EmbeddingError::HttpRequestFailed("503".to_string());
        assert!(err.to_string().contains("http request failed"));
    }
}
