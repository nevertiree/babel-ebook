//! Text chunking utilities based on token counts.

/// Return the number of tokens in `text` using the `cl100k_base` tokenizer.
///
/// # Panics
///
/// Panics if the `cl100k_base` tokenizer cannot be loaded.
#[must_use]
pub fn count_tokens(text: &str) -> usize {
    tiktoken_rs::cl100k_base()
        .expect("cl100k_base tokenizer should be available")
        .encode_ordinary(text)
        .len()
}

/// Split `text` into chunks of at most `max_tokens` tokens.
///
/// This is used as a fallback when a single sentence exceeds the limit.
fn split_by_tokens(text: &str, max_tokens: usize) -> Vec<String> {
    let bpe = tiktoken_rs::cl100k_base().expect("cl100k_base tokenizer should be available");
    let tokens = bpe.encode_ordinary(text);
    tokens
        .chunks(max_tokens)
        .map(|chunk| bpe.decode(chunk.to_vec()).unwrap_or_default())
        .collect()
}

/// Split `text` into chunks not exceeding `max_tokens`, keeping sentence
/// boundaries.
///
/// Long sentences that exceed `max_tokens` on their own are split at token
/// boundaries as a fallback.
#[must_use]
pub fn split_text_chunks(text: &str, max_tokens: usize) -> Vec<String> {
    let sentences = split_sentences(text);
    let mut chunks: Vec<String> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_tokens: usize = 0;

    for sent in sentences {
        let sent_tokens = count_tokens(&sent);

        if sent_tokens > max_tokens {
            if !current.is_empty() {
                chunks.push(current.join(" "));
                current.clear();
                current_tokens = 0;
            }
            chunks.extend(split_by_tokens(&sent, max_tokens));
            continue;
        }

        if !current.is_empty() && current_tokens + sent_tokens > max_tokens {
            chunks.push(current.join(" "));
            current.clear();
            current_tokens = 0;
        }

        current.push(sent);
        current_tokens += sent_tokens;
    }

    if !current.is_empty() {
        chunks.push(current.join(" "));
    }

    chunks
}

/// Split `text` into sentences.
///
/// This replicates Python's `(?<=[.!?。！？\n])\s+` regex split: a sentence
/// ends with one of the terminal characters followed by whitespace, and the
/// whitespace is consumed as the separator.
fn split_sentences(text: &str) -> Vec<String> {
    const END_CHARS: &[char] = &['.', '!', '?', '。', '！', '？', '\n'];
    let chars: Vec<char> = text.chars().collect();
    let mut sentences: Vec<String> = Vec::new();
    let mut start: usize = 0;
    let mut i: usize = 0;

    while i < chars.len() {
        if END_CHARS.contains(&chars[i]) {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j > i + 1 {
                let sentence: String = chars[start..j].iter().collect();
                let trimmed = sentence.trim();
                if !trimmed.is_empty() {
                    sentences.push(trimmed.to_string());
                }
                start = j;
                i = j;
                continue;
            }
        }
        i += 1;
    }

    if start < chars.len() {
        let remaining: String = chars[start..].iter().collect();
        let trimmed = remaining.trim();
        if !trimmed.is_empty() {
            sentences.push(trimmed.to_string());
        }
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_tokens_basic() {
        assert!(count_tokens("hello world") > 0);
    }

    #[test]
    fn split_sentences_preserves_punctuation() {
        let sentences = split_sentences("First. Second! Third?");
        assert_eq!(sentences, vec!["First.", "Second!", "Third?"]);
    }
}
