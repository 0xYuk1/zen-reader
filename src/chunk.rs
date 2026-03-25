use serde::Serialize;

const MAX_SENTENCES_PER_CHUNK: usize = 5;
const TARGET_SENTENCES_PER_CHUNK: usize = 4;

#[derive(Debug, Clone, Serialize)]
pub struct BookChunks {
    pub title: String,
    pub author: String,
    pub total_chunks: usize,
    pub chapters: Vec<ChapterInfo>,
    pub chunks: Vec<Chunk>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChapterInfo {
    pub title: String,
    pub start_chunk: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Chunk {
    pub index: usize,
    pub text: String,
    pub chapter: String,
}

/// Split a paragraph into sentences, handling CJK and ellipsis.
fn split_sentences(text: &str) -> Vec<String> {
    // Replace ellipsis with placeholder to avoid splitting on them
    let text = text.replace("...", "\x00ELLIPSIS\x00");
    let text = text.replace('…', "\x00ELLIPSIS\x00");

    let mut sentences = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut i = 0;
    while i < len {
        let c = chars[i];
        current.push(c);

        let is_sentence_end = match c {
            // CJK sentence endings — no trailing space needed
            '。' | '？' | '！' => true,
            // Western sentence endings — need trailing space or end of text
            '.' | '?' | '!' => {
                let next_is_space_or_end = i + 1 >= len || chars[i + 1].is_whitespace();
                // Check it's not an abbreviation (simple heuristic: single uppercase letter before dot)
                let is_abbreviation = if c == '.' && i >= 1 {
                    let prev = chars[i - 1];
                    // Mr. Dr. etc. — single letter or common abbreviations
                    prev.is_uppercase() && (i < 2 || !chars[i - 2].is_alphanumeric())
                } else {
                    false
                };
                next_is_space_or_end && !is_abbreviation
            }
            _ => false,
        };

        if is_sentence_end {
            // Consume trailing whitespace
            while i + 1 < len && chars[i + 1].is_whitespace() {
                i += 1;
            }
            let s = current.trim().to_string();
            if !s.is_empty() {
                sentences.push(s);
            }
            current.clear();
        }

        i += 1;
    }

    // Push remaining text
    let s = current.trim().to_string();
    if !s.is_empty() {
        sentences.push(s);
    }

    // Restore ellipsis
    sentences
        .into_iter()
        .map(|s| s.replace("\x00ELLIPSIS\x00", "..."))
        .collect()
}

/// Minimum character length for a chunk to stand on its own.
/// Shorter chunks get merged with the next one.
const MIN_CHUNK_CHARS: usize = 80;

/// Take a list of paragraphs for a chapter and produce chunks.
pub fn chunk_paragraphs(paragraphs: &[String], chapter_title: &str, start_index: usize) -> Vec<Chunk> {
    // Step 1: produce raw chunks from paragraphs
    let mut raw: Vec<String> = Vec::new();

    for para in paragraphs {
        let trimmed = para.trim();
        if trimmed.is_empty() {
            continue;
        }

        let sentences = split_sentences(trimmed);

        if sentences.len() <= MAX_SENTENCES_PER_CHUNK {
            raw.push(sentences.join(" "));
        } else {
            for group in sentences.chunks(TARGET_SENTENCES_PER_CHUNK) {
                let text = group.join(" ");
                if !text.is_empty() {
                    raw.push(text);
                }
            }
        }
    }

    // Step 2: merge consecutive short chunks together
    let mut merged: Vec<String> = Vec::new();
    let mut buffer = String::new();

    for text in &raw {
        if buffer.is_empty() {
            buffer = text.clone();
        } else {
            buffer.push('\n');
            buffer.push_str(text);
        }

        // Flush buffer if it's long enough and the next item isn't too short
        if buffer.len() >= MIN_CHUNK_CHARS {
            merged.push(buffer.clone());
            buffer.clear();
        }
    }
    // Flush remaining buffer
    if !buffer.is_empty() {
        // If there's a previous chunk and the leftover is tiny, append to it
        if buffer.len() < MIN_CHUNK_CHARS && !merged.is_empty() {
            let last = merged.last_mut().unwrap();
            last.push('\n');
            last.push_str(&buffer);
        } else {
            merged.push(buffer);
        }
    }

    // Step 3: assign indices
    let mut chunks = Vec::new();
    let mut index = start_index;
    for text in merged {
        chunks.push(Chunk {
            index,
            text,
            chapter: chapter_title.to_string(),
        });
        index += 1;
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sentences_english() {
        let sentences = split_sentences("Hello world. How are you? I'm fine!");
        assert_eq!(sentences, vec!["Hello world.", "How are you?", "I'm fine!"]);
    }

    #[test]
    fn test_split_sentences_ellipsis() {
        let sentences = split_sentences("He thought... then spoke. She agreed.");
        assert_eq!(sentences, vec!["He thought... then spoke.", "She agreed."]);
    }

    #[test]
    fn test_split_sentences_cjk() {
        let sentences = split_sentences("你好世界。今天天氣很好？是的！");
        assert_eq!(sentences, vec!["你好世界。", "今天天氣很好？", "是的！"]);
    }

    #[test]
    fn test_chunk_paragraphs_short() {
        let paras = vec!["A short paragraph.".to_string()];
        let chunks = chunk_paragraphs(&paras, "Ch 1", 0);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "A short paragraph.");
    }
}
