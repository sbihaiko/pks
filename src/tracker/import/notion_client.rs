use serde::Deserialize;

const NOTION_API_BASE: &str = "https://api.notion.com/v1";
const NOTION_VERSION: &str = "2022-06-28";

#[derive(Debug, Deserialize)]
pub struct NotionPage {
    pub id: String,
    pub properties: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct NotionBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct BlocksResponse {
    results: Vec<NotionBlock>,
}

pub async fn fetch_page(page_id: &str, token: &str) -> Result<NotionPage, String> {
    let url = format!("{NOTION_API_BASE}/pages/{page_id}");
    let resp = reqwest::Client::new()
        .get(&url)
        .bearer_auth(token)
        .header("Notion-Version", NOTION_VERSION)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    resp.json::<NotionPage>().await.map_err(|e| e.to_string())
}

pub async fn fetch_blocks(page_id: &str, token: &str) -> Result<Vec<NotionBlock>, String> {
    let url = format!("{NOTION_API_BASE}/blocks/{page_id}/children");
    let resp = reqwest::Client::new()
        .get(&url)
        .bearer_auth(token)
        .header("Notion-Version", NOTION_VERSION)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(resp.text().await.unwrap_or_default());
    }
    let r = resp.json::<BlocksResponse>().await.map_err(|e| e.to_string())?;
    Ok(r.results)
}

pub fn page_title(page: &NotionPage) -> String {
    page.properties
        .get("title")
        .or_else(|| page.properties.get("Name"))
        .and_then(|t| t.get("title"))
        .and_then(|arr| arr.get(0))
        .and_then(|item| item.get("plain_text"))
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string()
}

pub fn page_status(page: &NotionPage) -> String {
    page.properties
        .get("Status")
        .and_then(|s| s.get("status"))
        .and_then(|s| s.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_lowercase()
        .replace(' ', "_")
}

pub fn blocks_to_markdown(blocks: &[NotionBlock]) -> String {
    blocks.iter().map(block_to_line).collect::<Vec<_>>().join("\n")
}

fn block_to_line(block: &NotionBlock) -> String {
    let texts = block.data.get(&block.block_type)
        .and_then(|b| b.get("rich_text"))
        .and_then(|arr| arr.as_array());

    let text = match texts {
        Some(arr) => arr.iter()
            .filter_map(|item| item.get("plain_text").and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join(""),
        None => return String::new(),
    };

    match block.block_type.as_str() {
        "heading_1" => format!("# {text}"),
        "heading_2" => format!("## {text}"),
        "heading_3" => format!("### {text}"),
        "bulleted_list_item" => format!("- {text}"),
        "numbered_list_item" => format!("1. {text}"),
        "code" => format!("```\n{text}\n```"),
        _ => text,
    }
}
