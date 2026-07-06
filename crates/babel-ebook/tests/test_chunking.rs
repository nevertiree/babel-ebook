use babel_ebook::chunking::{count_tokens, split_text_chunks};

#[test]
fn count_tokens_is_positive_for_non_empty_text() {
    assert!(count_tokens("hello world") > 0);
    assert!(count_tokens("hello world") > count_tokens("hello"));
}

#[test]
fn short_text_is_not_split() {
    let text = "Hello world. This is a test.";
    let chunks = split_text_chunks(text, 100);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
}

#[test]
fn long_text_splits_by_token_limit() {
    // Build a long string that is well over a small token budget.
    let sentence = "The quick brown fox jumps over the lazy dog. ";
    let text = sentence.repeat(50);

    let chunks = split_text_chunks(&text, 20);

    assert!(
        chunks.len() > 1,
        "expected multiple chunks, got {}",
        chunks.len()
    );
    for chunk in &chunks {
        assert!(
            count_tokens(chunk) <= 20,
            "chunk exceeded max_tokens: {} tokens in {:?}",
            count_tokens(chunk),
            chunk
        );
    }
}

#[test]
fn single_long_sentence_splits_at_token_boundary() {
    // A single sentence with no punctuation boundary that exceeds the budget.
    let text = "a".repeat(200);

    let chunks = split_text_chunks(&text, 10);

    assert!(
        chunks.len() > 1,
        "expected fallback split, got {}",
        chunks.len()
    );
    for chunk in &chunks {
        assert!(
            count_tokens(chunk) <= 10,
            "chunk exceeded max_tokens: {} tokens in {:?}",
            count_tokens(chunk),
            chunk
        );
    }
}

#[test]
fn empty_text_yields_no_chunks() {
    assert!(split_text_chunks("", 10).is_empty());
    assert!(split_text_chunks("   \n\n   ", 10).is_empty());
}

#[test]
fn whitespace_is_normalized_between_sentences() {
    let text = "First sentence.   Second sentence.";
    let chunks = split_text_chunks(text, 100);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "First sentence. Second sentence.");
}
