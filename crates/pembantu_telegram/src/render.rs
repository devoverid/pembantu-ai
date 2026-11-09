//! Markdown (as produced by the LLM) → Telegram-flavoured HTML, plus message chunking.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

/// Telegram limits messages to 4096 characters; chunks leave room for HTML markup.
pub const MAX_CHUNK_CHARS: usize = 3500;

fn char_len(s: &str) -> usize {
    s.chars().count()
}

fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// Splits Markdown into chunks of at most `max_chars` characters, preferring blank-line
/// boundaries and never breaking inside a fenced code block unless the block itself is too long
/// (then the fence is closed and reopened around each piece).
pub fn split_markdown(md: &str, max_chars: usize) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut in_fence = false;
    for line in md.lines() {
        if is_fence(line) {
            in_fence = !in_fence;
        } else if !in_fence && line.trim().is_empty() {
            if !current.trim().is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            current.clear();
            continue;
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }
    if !current.trim().is_empty() {
        blocks.push(current);
    }

    let mut chunks = Vec::new();
    let mut chunk = String::new();
    for block in blocks {
        let pieces = if char_len(&block) > max_chars { split_block(&block, max_chars) } else { vec![block] };
        for piece in pieces {
            if chunk.is_empty() {
                chunk = piece;
            } else if char_len(&chunk) + 2 + char_len(&piece) <= max_chars {
                chunk.push_str("\n\n");
                chunk.push_str(&piece);
            } else {
                chunks.push(std::mem::replace(&mut chunk, piece));
            }
        }
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}

/// Splits one oversized block on line boundaries (hard-splitting overlong lines).
fn split_block(block: &str, max_chars: usize) -> Vec<String> {
    let lines: Vec<&str> = block.lines().collect();
    let fenced = lines.len() >= 2 && is_fence(lines[0]);
    if !fenced {
        return pack_lines(&lines, max_chars);
    }
    let opener = lines[0];
    let has_closer = is_fence(lines[lines.len() - 1]);
    let body = &lines[1..lines.len() - usize::from(has_closer)];
    // Each piece becomes "{opener}\n{body}\n```".
    let budget = max_chars.saturating_sub(char_len(opener) + 5).max(1);
    pack_lines(body, budget)
        .into_iter()
        .map(|piece| format!("{opener}\n{piece}\n```"))
        .collect()
}

fn pack_lines(lines: &[&str], max_chars: usize) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut piece = String::new();
    for line in lines {
        let mut segments = Vec::new();
        if char_len(line) > max_chars {
            let chars: Vec<char> = line.chars().collect();
            segments.extend(chars.chunks(max_chars).map(|c| c.iter().collect::<String>()));
        } else {
            segments.push((*line).to_owned());
        }
        for segment in segments {
            if piece.is_empty() {
                piece = segment;
            } else if char_len(&piece) + 1 + char_len(&segment) <= max_chars {
                piece.push('\n');
                piece.push_str(&segment);
            } else {
                pieces.push(std::mem::replace(&mut piece, segment));
            }
        }
    }
    if !piece.is_empty() {
        pieces.push(piece);
    }
    pieces
}

fn escape_into(out: &mut String, text: &str, attribute: bool) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
}

fn trim_trailing_whitespace(out: &mut String) {
    let len = out.trim_end().len();
    out.truncate(len);
}

/// Converts Markdown to the HTML subset accepted by Telegram's `ParseMode::Html`.
pub fn markdown_to_telegram_html(md: &str) -> String {
    let mut out = String::with_capacity(md.len() + md.len() / 4);
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut fenced_code = false;

    for event in Parser::new_ext(md, Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { .. } => out.push_str("<b>"),
                Tag::BlockQuote(_) => out.push_str("<blockquote>"),
                Tag::CodeBlock(CodeBlockKind::Fenced(lang)) => {
                    fenced_code = true;
                    let lang = lang.split_whitespace().next().unwrap_or("");
                    if lang.is_empty() {
                        out.push_str("<pre><code>");
                    } else {
                        out.push_str("<pre><code class=\"language-");
                        escape_into(&mut out, lang, true);
                        out.push_str("\">");
                    }
                }
                Tag::CodeBlock(CodeBlockKind::Indented) => {
                    fenced_code = false;
                    out.push_str("<pre>");
                }
                Tag::List(start) => lists.push(start),
                Tag::Item => {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    for _ in 1..lists.len() {
                        out.push_str("  ");
                    }
                    match lists.last_mut() {
                        Some(Some(n)) => {
                            out.push_str(&format!("{n}. "));
                            *n += 1;
                        }
                        _ => out.push_str("• "),
                    }
                }
                Tag::Emphasis => out.push_str("<i>"),
                Tag::Strong => out.push_str("<b>"),
                Tag::Strikethrough => out.push_str("<s>"),
                Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                    out.push_str("<a href=\"");
                    escape_into(&mut out, &dest_url, true);
                    out.push_str("\">");
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => out.push_str(if lists.is_empty() { "\n\n" } else { "\n" }),
                TagEnd::Heading(_) => out.push_str("</b>\n\n"),
                TagEnd::BlockQuote(_) => {
                    trim_trailing_whitespace(&mut out);
                    out.push_str("</blockquote>\n\n");
                }
                TagEnd::CodeBlock => {
                    if out.ends_with('\n') {
                        out.pop();
                    }
                    out.push_str(if fenced_code { "</code></pre>\n\n" } else { "</pre>\n\n" });
                }
                TagEnd::List(_) => {
                    lists.pop();
                    if lists.is_empty() {
                        trim_trailing_whitespace(&mut out);
                        out.push_str("\n\n");
                    }
                }
                TagEnd::Emphasis => out.push_str("</i>"),
                TagEnd::Strong => out.push_str("</b>"),
                TagEnd::Strikethrough => out.push_str("</s>"),
                TagEnd::Link | TagEnd::Image => out.push_str("</a>"),
                _ => {}
            },
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => escape_into(&mut out, &text, false),
            Event::Code(code) => {
                out.push_str("<code>");
                escape_into(&mut out, &code, false);
                out.push_str("</code>");
            }
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("\n———\n"),
            _ => {}
        }
    }
    out.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_inline_html_and_renders_bold() {
        assert_eq!(markdown_to_telegram_html("**a** <b>"), "<b>a</b> &lt;b&gt;");
    }

    #[test]
    fn fenced_code_is_escaped_and_tagged() {
        let html = markdown_to_telegram_html("Look:\n\n```rust\nif a < b { x() }\n```\n\nDone");
        assert_eq!(
            html,
            "Look:\n\n<pre><code class=\"language-rust\">if a &lt; b { x() }</code></pre>\n\nDone"
        );
    }

    #[test]
    fn nested_and_ordered_lists() {
        let html = markdown_to_telegram_html("Items:\n\n- a\n  - b\n- c\n\n1. one\n2. two");
        assert_eq!(html, "Items:\n\n• a\n  • b\n• c\n\n1. one\n2. two");
    }

    #[test]
    fn link_href_is_attribute_escaped() {
        assert_eq!(
            markdown_to_telegram_html("[x](https://e.com/?a=1&b=\"2\")"),
            "<a href=\"https://e.com/?a=1&amp;b=&quot;2&quot;\">x</a>"
        );
    }

    #[test]
    fn long_fenced_block_splits_on_lines_with_balanced_fences() {
        let body: Vec<String> = (0..100).map(|i| format!("line {i:03} {}", "x".repeat(30))).collect();
        let md = format!("Intro\n\n```text\n{}\n```\n\nOutro", body.join("\n"));
        assert!(char_len(&md) > 4000);

        let chunks = split_markdown(&md, MAX_CHUNK_CHARS);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| char_len(c) <= MAX_CHUNK_CHARS));

        let mut seen = Vec::new();
        for chunk in &chunks {
            let fences = chunk.lines().filter(|l| is_fence(l)).count();
            assert_eq!(fences % 2, 0, "unbalanced fence in chunk: {chunk}");
            seen.extend(chunk.lines().filter(|l| l.starts_with("line ")).map(str::to_owned));
        }
        assert_eq!(seen, body);
        assert!(chunks[0].starts_with("Intro\n\n```text\n"));
        assert!(chunks.last().unwrap().ends_with("```\n\nOutro"));
    }

    #[test]
    fn empty_input_yields_no_chunks() {
        assert!(split_markdown("", MAX_CHUNK_CHARS).is_empty());
        assert!(split_markdown(" \n\n \n", MAX_CHUNK_CHARS).is_empty());
        assert_eq!(markdown_to_telegram_html(""), "");
    }
}
