use std::collections::HashMap;
use std::env;
use std::fs;

#[derive(Debug, Clone, PartialEq)]
pub enum TrackerType { Notion }

#[derive(Debug)]
pub struct ExportRequest {
    pub file_path: String,
    pub tracker_type: Option<TrackerType>,
}

#[derive(Debug)]
pub struct ExportedPage {
    pub page_id: String,
    pub synced_at: String,
}

#[derive(Debug)]
pub enum ExportError {
    FrontmatterMissing,
    CollisionDetected { remote_updated_at: String },
    ApiError(String),
    FileNotFound,
}

#[derive(Debug)]
pub struct Frontmatter {
    pub fields: HashMap<String, String>,
    pub body: String,
}

pub fn parse_frontmatter(raw: &str) -> Result<Frontmatter, ExportError> {
    let stripped = raw.trim_start();
    if !stripped.starts_with("---") { return Err(ExportError::FrontmatterMissing); }
    let after = &stripped[3..];
    let end = after.find("---").ok_or(ExportError::FrontmatterMissing)?;
    let fields = after[..end].lines()
        .filter_map(|l| { let mut p = l.splitn(2, ':'); let k = p.next()?.trim().to_string(); let v = p.next()?.trim().to_string(); if k.is_empty() { return None; } Some((k, v)) })
        .collect();
    Ok(Frontmatter { fields, body: after[end + 3..].trim_start().to_string() })
}

pub fn detect_collision(fm: &Frontmatter, remote: &str) -> Result<(), ExportError> {
    let local = fm.fields.get("updated_at").map(|s| s.as_str()).unwrap_or("");
    if !remote.is_empty() && remote > local {
        return Err(ExportError::CollisionDetected { remote_updated_at: remote.to_string() });
    }
    Ok(())
}

pub fn resolve_tracker(fm: &Frontmatter, override_type: Option<TrackerType>) -> Result<TrackerType, ExportError> {
    if let Some(t) = override_type { return Ok(t); }
    match fm.fields.get("tracker").map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("notion") => Ok(TrackerType::Notion),
        _ => Err(ExportError::FrontmatterMissing),
    }
}

fn inject_sync_fields(raw: &str, synced_at: &str, page_id: &str) -> String {
    let after = raw.trim_start().trim_start_matches("---");
    let end = after.find("---").unwrap_or(after.len());
    let mut lines: Vec<String> = after[..end].lines()
        .filter(|l| { let k = l.splitn(2, ':').next().unwrap_or("").trim(); k != "synced_at" && k != "notion_page_id" })
        .map(|l| l.to_string()).collect();
    lines.push(format!("synced_at: {}", synced_at));
    lines.push(format!("notion_page_id: {}", page_id));
    format!("---\n{}---{}", lines.join("\n") + "\n", &after[end + 3..])
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let s = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    let (d, t) = (s / 86400, s % 86400);
    let (y, mo, day) = days_to_ymd(d);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, day, t/3600, (t%3600)/60, t%60)
}

fn is_leap(y: u64) -> bool { (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 }
fn days_in_year(y: u64) -> u64 { match is_leap(y) { true => 366, false => 365 } }
fn dim(y: u64, m: u64) -> u64 { match m { 1|3|5|7|8|10|12 => 31, 4|6|9|11 => 30, 2 if is_leap(y) => 29, _ => 28 } }

fn days_to_ymd(mut d: u64) -> (u64, u64, u64) {
    let mut y = 1970u64;
    while d >= days_in_year(y) { d -= days_in_year(y); y += 1; }
    let mut mo = 1u64;
    while d >= dim(y, mo) { d -= dim(y, mo); mo += 1; }
    (y, mo, d + 1)
}

fn notion_token() -> Result<String, ExportError> {
    env::var("NOTION_TOKEN").map_err(|_| ExportError::ApiError("NOTION_TOKEN not set".into()))
}

async fn fetch_remote_updated_at(page_id: &str, token: &str) -> Result<String, ExportError> {
    let body: serde_json::Value = reqwest::Client::new()
        .get(format!("https://api.notion.com/v1/pages/{}", page_id))
        .bearer_auth(token).header("Notion-Version", "2022-06-28")
        .send().await.map_err(|e| ExportError::ApiError(e.to_string()))?
        .json().await.map_err(|e| ExportError::ApiError(e.to_string()))?;
    Ok(body["last_edited_time"].as_str().unwrap_or("").to_string())
}

async fn publish_to_notion(page_id: Option<&str>, title: &str, content: &str, token: &str) -> Result<String, ExportError> {
    let block = serde_json::json!({"object":"block","type":"paragraph","paragraph":{"rich_text":[{"type":"text","text":{"content":content}}]}});
    let tp = serde_json::json!({"title":[{"type":"text","text":{"content":title}}]});
    let payload = match page_id {
        Some(_) => serde_json::json!({"properties":{"title":tp},"children":[block]}),
        None => { let pid = env::var("NOTION_PARENT_PAGE_ID").unwrap_or_default(); serde_json::json!({"parent":{"page_id":pid},"properties":{"title":tp},"children":[block]}) }
    };
    let client = reqwest::Client::new();
    let req = match page_id { Some(id) => client.patch(format!("https://api.notion.com/v1/pages/{}", id)), None => client.post("https://api.notion.com/v1/pages") };
    let body: serde_json::Value = req.bearer_auth(token).header("Notion-Version", "2022-06-28").json(&payload)
        .send().await.map_err(|e| ExportError::ApiError(e.to_string()))?
        .json().await.map_err(|e| ExportError::ApiError(e.to_string()))?;
    Ok(body["id"].as_str().ok_or_else(|| ExportError::ApiError("no id in response".into()))?.to_string())
}

pub async fn export_to_tracker(request: ExportRequest) -> Result<ExportedPage, ExportError> {
    let raw = fs::read_to_string(&request.file_path).map_err(|_| ExportError::FileNotFound)?;
    let fm = parse_frontmatter(&raw)?;
    let _tracker = resolve_tracker(&fm, request.tracker_type)?;
    let token = notion_token()?;
    let existing = fm.fields.get("notion_page_id").map(|s| s.as_str());
    if let Some(pid) = existing { detect_collision(&fm, &fetch_remote_updated_at(pid, &token).await?)?; }
    let title = fm.fields.get("title").map(|s| s.as_str()).unwrap_or("Untitled");
    let page_id = publish_to_notion(existing, title, &fm.body, &token).await?;
    let synced_at = now_iso8601();
    fs::write(&request.file_path, inject_sync_fields(&raw, &synced_at, &page_id))
        .map_err(|e| ExportError::ApiError(e.to_string()))?;
    Ok(ExportedPage { page_id, synced_at })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fm(raw: &str) -> Frontmatter { parse_frontmatter(raw).unwrap() }

    #[test]
    fn detect_collision_newer_remote_fails() {
        assert!(matches!(
            detect_collision(&fm("---\nupdated_at: 2024-01-01T00:00:00Z\ntracker: notion\n---\nbody"), "2025-01-01T00:00:00Z").unwrap_err(),
            ExportError::CollisionDetected { .. }
        ));
    }

    #[test]
    fn detect_collision_older_remote_passes() {
        assert!(detect_collision(&fm("---\nupdated_at: 2025-06-01T00:00:00Z\ntracker: notion\n---\nbody"), "2024-01-01T00:00:00Z").is_ok());
    }

    #[test]
    fn detect_collision_empty_remote_passes() {
        assert!(detect_collision(&fm("---\nupdated_at: 2024-01-01T00:00:00Z\ntracker: notion\n---\nbody"), "").is_ok());
    }

    #[test]
    fn parse_frontmatter_extracts_fields_and_body() {
        let f = fm("---\ntitle: T\ntracker: notion\n---\n# Body");
        assert_eq!(f.fields["title"], "T");
        assert_eq!(f.body.trim(), "# Body");
    }

    #[test]
    fn parse_frontmatter_missing_delimiter_fails() {
        assert!(matches!(parse_frontmatter("no delimiters").unwrap_err(), ExportError::FrontmatterMissing));
    }

    #[test]
    fn resolve_tracker_from_frontmatter_notion() {
        assert_eq!(resolve_tracker(&fm("---\ntracker: notion\n---\nbody"), None).unwrap(), TrackerType::Notion);
    }

    #[test]
    fn resolve_tracker_override_wins() {
        assert_eq!(resolve_tracker(&fm("---\ntracker: unknown\n---\nbody"), Some(TrackerType::Notion)).unwrap(), TrackerType::Notion);
    }

    #[test]
    fn resolve_tracker_missing_fails() {
        assert!(matches!(resolve_tracker(&fm("---\ntitle: T\n---\nbody"), None).unwrap_err(), ExportError::FrontmatterMissing));
    }

    #[test]
    fn inject_sync_fields_adds_and_replaces() {
        let u = inject_sync_fields("---\nsynced_at: old\nnotion_page_id: old-id\n---\nbody", "2025-01-01T00:00:00Z", "new-id");
        assert!(u.contains("synced_at: 2025-01-01T00:00:00Z") && u.contains("notion_page_id: new-id") && !u.contains("old\n"));
    }

    #[test]
    fn export_returns_file_not_found_for_missing_path() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        assert!(matches!(
            rt.block_on(export_to_tracker(ExportRequest { file_path: "/tmp/nonexistent_pks_xyz_abc.md".to_string(), tracker_type: None })).unwrap_err(),
            ExportError::FileNotFound
        ));
    }
}
