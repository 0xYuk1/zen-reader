use crate::chunk::{BookChunks, ChapterInfo, Chunk, chunk_paragraphs};
use epub::doc::EpubDoc;
use scraper::{Html, Selector};
use std::io::Cursor;

/// Parse EPUB bytes into chunked content for the zen reader.
pub fn parse_epub(data: &[u8]) -> Result<BookChunks, String> {
    let cursor = Cursor::new(data.to_vec());
    let mut doc = EpubDoc::from_reader(cursor).map_err(|e| format!("Failed to open EPUB: {e}"))?;

    let title = doc
        .mdata("title")
        .map(|m| m.value.clone())
        .unwrap_or_else(|| "Unknown Title".to_string());
    let author = doc
        .mdata("creator")
        .map(|m| m.value.clone())
        .unwrap_or_else(|| "Unknown Author".to_string());

    let mut all_chunks: Vec<Chunk> = Vec::new();
    let mut chapters: Vec<ChapterInfo> = Vec::new();
    let mut chunk_index = 0;

    // Iterate through spine (reading order)
    let num_chapters = doc.get_num_chapters();
    for ch in 0..num_chapters {
        doc.set_current_chapter(ch);

        let chapter_title = doc
            .get_current_id()
            .unwrap_or_else(|| format!("Section {}", ch + 1));

        // Get the HTML content of this chapter
        let content = match doc.get_current_str() {
            Some((html, _mime)) => html,
            None => continue,
        };

        let paragraphs = extract_paragraphs(&content);
        if paragraphs.is_empty() {
            continue;
        }

        chapters.push(ChapterInfo {
            title: chapter_title.clone(),
            start_chunk: chunk_index,
        });

        let new_chunks = chunk_paragraphs(&paragraphs, &chapter_title, chunk_index);
        chunk_index += new_chunks.len();
        all_chunks.extend(new_chunks);
    }

    Ok(BookChunks {
        title,
        author,
        total_chunks: all_chunks.len(),
        chapters,
        chunks: all_chunks,
    })
}

/// Extract paragraph text from HTML content.
fn extract_paragraphs(html: &str) -> Vec<String> {
    let document = Html::parse_document(html);

    // Try block-level elements that typically contain paragraph text
    let selectors = ["p", "blockquote", "li", "h1", "h2", "h3", "h4", "h5", "h6"];

    let mut paragraphs = Vec::new();

    for sel_str in &selectors {
        if let Ok(selector) = Selector::parse(sel_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join("");
                let text = normalize_whitespace(&text);
                if !text.is_empty() && text.len() > 1 {
                    paragraphs.push(text);
                }
            }
        }
    }

    // If no paragraphs found with standard tags, try divs with direct text
    if paragraphs.is_empty() {
        if let Ok(selector) = Selector::parse("div") {
            for element in document.select(&selector) {
                let text: String = element
                    .text()
                    .collect::<Vec<_>>()
                    .join("");
                let text = normalize_whitespace(&text);
                if !text.is_empty() && text.len() > 10 {
                    paragraphs.push(text);
                }
            }
        }
    }

    paragraphs
}

/// Normalize whitespace: collapse runs of whitespace into single spaces, trim.
fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}
