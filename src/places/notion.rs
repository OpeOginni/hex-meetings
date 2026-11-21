//! Notion: a new page under the page the person picked, made from the
//! agent's Markdown. Notion's API takes blocks, not Markdown, so this turns
//! the common parts (headings, paragraphs, lists, to-dos, quotes, code,
//! dividers, **bold**, `code`, links) into blocks; anything else stays text.

use serde_json::{Value, json};

/// Notion caps a text run at 2,000 characters and a request at 100 blocks.
const TEXT_LIMIT: usize = 2000;
pub const BLOCKS_PER_REQUEST: usize = 100;

/// The page id in a Notion link or a bare id: the link's last part, without
/// dashes, ends in the 32 hex characters of the id.
pub fn page_id(link: &str) -> Option<String> {
    let path = link.trim().split(['?', '#']).next()?;
    let last: String = path.rsplit('/').next()?.chars().filter(|c| *c != '-').collect();
    let id = last.get(last.len().checked_sub(32)?..)?;
    id.chars().all(|c| c.is_ascii_hexdigit()).then(|| id.to_lowercase())
}

/// Inline Markdown as Notion rich text: **bold**, `code` and [links](url).
pub fn rich_text(line: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut push = |text: &str, bold: bool, code: bool, link: Option<&str>| {
        for chunk in chunks(text) {
            let mut t = json!({ "type": "text", "text": { "content": chunk } });
            if let Some(url) = link {
                t["text"]["link"] = json!({ "url": url });
            }
            if bold || code {
                t["annotations"] = json!({ "bold": bold, "code": code });
            }
            out.push(t);
        }
    };
    let mut rest = line;
    let mut plain = String::new();
    while !rest.is_empty() {
        let found = [("**", "**"), ("`", "`"), ("[", "](")].iter().filter_map(|(open, _)| rest.find(open).map(|i| (i, *open))).min();
        let Some((at, open)) = found else {
            plain.push_str(rest);
            break;
        };
        plain.push_str(&rest[..at]);
        let after = &rest[at + open.len()..];
        let closed = match open {
            "**" => after.find("**").map(|end| (&after[..end], None, end + 2, true, false)),
            "`" => after.find('`').map(|end| (&after[..end], None, end + 1, false, true)),
            _ => after.find("](").and_then(|mid| {
                let url_end = after[mid + 2..].find(')')?;
                Some((&after[..mid], Some(&after[mid + 2..mid + 2 + url_end]), mid + 2 + url_end + 1, false, false))
            }),
        };
        match closed {
            Some((inner, link, used, bold, code)) => {
                if !plain.is_empty() {
                    push(&std::mem::take(&mut plain), false, false, None);
                }
                push(inner, bold, code, link);
                rest = &after[used..];
            }
            None => {
                plain.push_str(open);
                rest = after;
            }
        }
    }
    if !plain.is_empty() {
        push(&plain, false, false, None);
    }
    out
}

fn chunks(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars.chunks(TEXT_LIMIT).map(|c| c.iter().collect()).collect()
}

fn block(kind: &str, text: &str) -> Value {
    json!({ "object": "block", "type": kind, kind: { "rich_text": rich_text(text) } })
}

/// The agent's Markdown as Notion blocks.
pub fn blocks(markdown: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut code: Option<Vec<&str>> = None;
    let flush = |paragraph: &mut Vec<&str>, out: &mut Vec<Value>| {
        if !paragraph.is_empty() {
            out.push(block("paragraph", &paragraph.join(" ")));
            paragraph.clear();
        }
    };
    for line in markdown.lines() {
        if let Some(lines) = &mut code {
            if line.trim_start().starts_with("```") {
                let text = lines.join("\n");
                out.push(json!({ "object": "block", "type": "code", "code": {
                    "rich_text": chunks(&text).into_iter().map(|c| json!({ "type": "text", "text": { "content": c } })).collect::<Vec<_>>(),
                    "language": "plain text" } }));
                code = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        let t = line.trim();
        let numbered = t.split_once(". ").filter(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        let item = if t.is_empty() {
            None
        } else if t.starts_with("```") {
            code = Some(Vec::new());
            Some(None)
        } else if t == "---" || t == "***" {
            Some(Some(json!({ "object": "block", "type": "divider", "divider": {} })))
        } else if let Some(h) = t.strip_prefix("### ") {
            Some(Some(block("heading_3", h)))
        } else if let Some(h) = t.strip_prefix("## ") {
            Some(Some(block("heading_2", h)))
        } else if let Some(h) = t.strip_prefix("# ") {
            Some(Some(block("heading_1", h)))
        } else if let Some(rest) = t.strip_prefix("- [ ] ").or_else(|| t.strip_prefix("* [ ] ")) {
            Some(Some(json!({ "object": "block", "type": "to_do", "to_do": { "rich_text": rich_text(rest), "checked": false } })))
        } else if let Some(rest) = t.strip_prefix("- [x] ").or_else(|| t.strip_prefix("- [X] ")) {
            Some(Some(json!({ "object": "block", "type": "to_do", "to_do": { "rich_text": rich_text(rest), "checked": true } })))
        } else if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("• ")) {
            Some(Some(block("bulleted_list_item", rest)))
        } else if let Some((_, rest)) = numbered {
            Some(Some(block("numbered_list_item", rest)))
        } else if let Some(rest) = t.strip_prefix("> ") {
            Some(Some(block("quote", rest)))
        } else {
            // An indented line under a list item belongs to that item.
            let indented = line.starts_with([' ', '\t']) && paragraph.is_empty();
            let item = out.last_mut().and_then(|b: &mut Value| {
                let kind = b["type"].as_str()?.to_string();
                matches!(kind.as_str(), "bulleted_list_item" | "numbered_list_item" | "to_do").then_some((b, kind))
            });
            if let (true, Some((b, kind))) = (indented, item) {
                if let Some(runs) = b[&kind]["rich_text"].as_array_mut() {
                    runs.extend(rich_text(&format!(" {t}")));
                }
                continue;
            }
            paragraph.push(t);
            continue;
        };
        flush(&mut paragraph, &mut out);
        if let Some(Some(b)) = item {
            out.push(b);
        }
    }
    flush(&mut paragraph, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_id_comes_from_a_link_or_an_id() {
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(page_id(&format!("https://www.notion.so/acme/Sales-calls-{id}?pvs=4")).as_deref(), Some(id));
        assert_eq!(page_id("01234567-89ab-cdef-0123-456789abcdef").as_deref(), Some(id));
        assert_eq!(page_id("https://www.notion.so/acme/Sales-calls"), None);
    }

    #[test]
    fn markdown_becomes_notion_blocks() {
        let md = "## Summary\nNorthwind wants **20 seats**\nthis quarter.\n\n- [ ] You: send the `plan`\n- [x] Done\n- A [link](https://x.y)\n  with more on the next line\n1. First\n> Quote\n---\n```\nlet x = 1;\n```";
        let b = blocks(md);
        let kinds: Vec<&str> = b.iter().map(|b| b["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["heading_2", "paragraph", "to_do", "to_do", "bulleted_list_item", "numbered_list_item", "quote", "divider", "code"]);
        let para = &b[1]["paragraph"]["rich_text"];
        assert_eq!(para[1]["text"]["content"], "20 seats");
        assert_eq!(para[1]["annotations"]["bold"], true);
        assert_eq!(para[2]["text"]["content"], " this quarter.");
        assert_eq!(b[2]["to_do"]["checked"], false);
        assert_eq!(b[3]["to_do"]["checked"], true);
        assert_eq!(b[4]["bulleted_list_item"]["rich_text"][1]["text"]["link"]["url"], "https://x.y");
        assert_eq!(b[4]["bulleted_list_item"]["rich_text"][2]["text"]["content"], " with more on the next line");
        assert_eq!(b[8]["code"]["rich_text"][0]["text"]["content"], "let x = 1;");
        // A lone ** stays as written.
        assert_eq!(rich_text("2 ** 3")[0]["text"]["content"], "2 ** 3");
    }
}
