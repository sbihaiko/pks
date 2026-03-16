use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

pub struct Section {
    pub heading_hierarchy: Vec<String>,
    pub text: String,
}

pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    ((words as f64) * 1.3) as usize
}

fn heading_depth(level: HeadingLevel) -> usize {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn sliding_window(text: &str, max_tokens: usize, overlap_tokens: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let max_words = ((max_tokens as f64) / 1.3) as usize;
    let overlap_words = ((overlap_tokens as f64) / 1.3) as usize;
    let step = max_words.saturating_sub(overlap_words).max(1);

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < words.len() {
        let end = (start + max_words).min(words.len());
        chunks.push(words[start..end].join(" "));
        if end >= words.len() {
            break;
        }
        start += step;
    }

    chunks
}

pub fn parse_sections(content: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut hierarchy: Vec<(usize, String)> = Vec::new();
    let mut current_text = String::new();
    let mut current_heading_text = String::new();
    let mut current_heading_level: Option<usize> = None;
    let mut in_heading = false;
    let mut current_hierarchy: Vec<String> = Vec::new();

    let parser = Parser::new_ext(content, Options::all());

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                if !current_text.trim().is_empty() || !current_hierarchy.is_empty() {
                    sections.push(Section {
                        heading_hierarchy: current_hierarchy.clone(),
                        text: current_text.trim().to_string(),
                    });
                }
                current_text = String::new();
                current_heading_text = String::new();
                current_heading_level = Some(heading_depth(level));
                in_heading = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let depth = current_heading_level.unwrap_or(1);
                hierarchy.retain(|(d, _)| *d < depth);
                hierarchy.push((depth, current_heading_text.trim().to_string()));
                current_hierarchy = hierarchy.iter().map(|(_, h)| h.clone()).collect();
                current_heading_level = None;
            }
            Event::Text(text) => {
                if in_heading {
                    current_heading_text.push_str(&text);
                    continue;
                }
                current_text.push_str(&text);
                current_text.push(' ');
            }
            Event::SoftBreak | Event::HardBreak => {
                if !in_heading {
                    current_text.push(' ');
                }
            }
            Event::Code(code) => {
                if !in_heading {
                    current_text.push_str(&code);
                    current_text.push(' ');
                }
            }
            _ => {}
        }
    }

    if !current_text.trim().is_empty() || !current_hierarchy.is_empty() {
        sections.push(Section {
            heading_hierarchy: current_hierarchy,
            text: current_text.trim().to_string(),
        });
    }

    sections
}

pub fn merge_small_sections(sections: Vec<Section>, min_tokens: usize) -> Vec<Section> {
    let mut merged: Vec<Section> = Vec::new();

    for section in sections {
        let tokens = estimate_tokens(&section.text);

        if merged.is_empty() {
            merged.push(section);
            continue;
        }

        let last_tokens = estimate_tokens(&merged.last().unwrap().text);

        if tokens >= min_tokens || last_tokens >= min_tokens {
            merged.push(section);
            continue;
        }

        let last = merged.last_mut().unwrap();
        if !section.text.is_empty() {
            last.text.push(' ');
            last.text.push_str(&section.text);
        }
    }

    merged
}
