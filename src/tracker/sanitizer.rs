use std::env;

const DEFAULT_MAX_BYTES: usize = 1_048_576; // 1 MB

/// Returns the configured max import size in bytes.
/// Reads `PKS_IMPORT_MAX_SIZE` env var. Defaults to 1 MB.
pub fn max_import_size_bytes() -> usize {
    env::var("PKS_IMPORT_MAX_SIZE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(DEFAULT_MAX_BYTES)
}

/// Sanitize imported content: strip HTML, scripts, malicious links.
pub fn sanitize(content: &str) -> String {
    let step1 = remove_script_and_style_blocks(content);
    let step2 = neutralize_malicious_links(&step1);
    remove_inline_html(&step2)
}

/// Sanitize with a byte-size limit. Truncates if over `max_bytes`.
pub fn sanitize_with_limit(content: &str, max_bytes: usize) -> String {
    let truncated = truncate_to_limit(content, max_bytes);
    sanitize(&truncated)
}

fn truncate_to_limit(content: &str, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content.to_string();
    }
    tracing::warn!(
        "Content exceeds limit ({} > {}), truncating",
        content.len(), max_bytes
    );
    let mut end = max_bytes;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    content[..end].to_string()
}

fn remove_script_and_style_blocks(content: &str) -> String {
    let mut result = content.to_string();
    for tag in &["script", "style"] {
        result = remove_tag_block(&result, tag);
    }
    result
}

fn remove_tag_block(content: &str, tag: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut rest = content;
    while let Some(start) = find_ci(rest, &open) {
        result.push_str(&rest[..start]);
        let after = &rest[start..];
        match find_ci(after, &close) {
            Some(end) => rest = &after[end + close.len()..],
            None => { rest = ""; break; }
        }
    }
    result.push_str(rest);
    result
}

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack.to_ascii_lowercase().find(&needle.to_ascii_lowercase())
}

fn neutralize_malicious_links(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut rest = content;
    while let Some(pos) = rest.find("href=") {
        result.push_str(&rest[..pos]);
        let after = &rest[pos + 5..];
        let (q, inner) = match after.chars().next() {
            Some(q @ ('"' | '\'')) => (q, &after[1..]),
            _ => { result.push_str("href="); rest = after; continue; }
        };
        if let Some(end) = inner.find(q) {
            let url = &inner[..end];
            let low = url.trim().to_ascii_lowercase();
            result.push_str("href=");
            result.push(q);
            if !is_malicious_scheme(&low) { result.push_str(url); }
            result.push(q);
            rest = &inner[end + 1..];
        } else {
            result.push_str("href=");
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

fn is_malicious_scheme(url: &str) -> bool {
    ["javascript:", "data:", "vbscript:"]
        .iter()
        .any(|s| url.starts_with(s))
}

/// Remove inline HTML tags, preserving code fences and inline code.
fn remove_inline_html(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut in_fence = false;
    for (i, line) in content.split('\n').enumerate() {
        if i > 0 { result.push('\n'); }
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            result.push_str(line);
        } else if in_fence {
            result.push_str(line);
        } else {
            result.push_str(&strip_tags(line));
        }
    }
    result
}

fn strip_tags(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let (mut in_code, mut in_tag) = (false, false);
    for ch in line.chars() {
        if ch == '`' && !in_tag { in_code = !in_code; out.push(ch); continue; }
        if in_code { out.push(ch); continue; }
        if ch == '<' && !in_tag { in_tag = true; continue; }
        if ch == '>' && in_tag { in_tag = false; continue; }
        if !in_tag { out.push(ch); }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_removes_script_tags() {
        let input = "Hello<script>alert('xss')</script> World";
        assert_eq!(sanitize(input), "Hello World");
    }

    #[test]
    fn sanitize_removes_inline_html() {
        let input = "This is <b>bold</b> and <i>italic</i>";
        assert_eq!(sanitize(input), "This is bold and italic");
    }

    #[test]
    fn sanitize_removes_javascript_links() {
        let input = r#"<a href="javascript:alert('xss')">click</a>"#;
        let result = sanitize(input);
        assert!(!result.contains("javascript:"));
        assert!(result.contains("click"));
    }

    #[test]
    fn sanitize_preserves_markdown_links() {
        let input = "Check [this link](https://example.com) out";
        assert_eq!(sanitize(input), input);
    }

    #[test]
    fn sanitize_preserves_code_blocks() {
        let input = "```html\n<div>keep this</div>\n```";
        assert_eq!(sanitize(input), input);
    }

    #[test]
    fn sanitize_with_limit_truncates_oversized_content() {
        let input = "A".repeat(100);
        let result = sanitize_with_limit(&input, 50);
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn max_import_size_bytes_returns_one_mb_default() {
        env::remove_var("PKS_IMPORT_MAX_SIZE");
        assert_eq!(max_import_size_bytes(), 1_048_576);
    }
}
