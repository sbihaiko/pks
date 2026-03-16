pub async fn run_status(port: u16) -> i32 {
    let url = format!("http://127.0.0.1:{port}/health");
    let response = match reqwest::get(&url).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("pks status: could not reach daemon at {url}: {e}");
            return 1;
        }
    };
    let json: serde_json::Value = match response.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("pks status: failed to parse response as JSON: {e}");
            return 1;
        }
    };
    let pretty = match serde_json::to_string_pretty(&json) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pks status: failed to pretty-print JSON: {e}");
            return 1;
        }
    };
    println!("{pretty}");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn run_status_returns_one_when_daemon_unreachable() {
        let result = run_status(19999).await;
        assert_eq!(result, 1);
    }
}
