//! BabelEbook core library: EPUB translation pipeline with caching,
//! chunking, and pluggable LLM providers.

// `warn` keeps `cargo test` passing while surfacing missing docs; Task 1.2 will
// add the documentation and switch this to `#![deny(missing_docs)]`.
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::doc_markdown)]
#![warn(clippy::nursery)]
#![warn(clippy::perf)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod cache;
pub mod chunking;
pub mod config;
pub mod core;
pub mod epub;
pub mod html;
pub mod input_formats;
pub mod translator;

pub use cache::TranslationCache;
pub use config::{
    provider_env_var, Config, GlossaryEntry, OutputMode, ProviderConfig, TranslationScope,
    TranslationStyle, KNOWN_PROVIDERS,
};
pub use core::{
    estimate_source_tokens, translatable_chapters, translate_epub, BabelEbookError,
    ProgressCallback, ProgressEvent,
};
pub use epub::{
    read_epub, should_translate_doc, write_epub, Chapter, EpubBook, EpubMetadata, Resource,
};
pub use html::{process_document, translate_text};
pub use input_formats::{read_input_book, supported_extensions};
pub use translator::{get_translator, TranslateContext, Translator};

// Load translations from `crates/babel-ebook/locales`. The `i18n!` macro also
// provides the `t!` translation macro and `set_locale` helper.
rust_i18n::i18n!("locales", fallback = "en");

// Re-export the translation macro and locale setter so consumers can use
// `babel_ebook::t!("key")` and `babel_ebook::set_locale("zh-CN")`.
pub use rust_i18n::{set_locale, t};

#[cfg(test)]
mod tests {
    use crate::{set_locale, t};

    /// Verify that the bundled locale files load for at least English and
    /// Simplified Chinese.
    #[test]
    fn i18n_loads_all_locales() {
        set_locale("en");
        assert!(!t!("hello").is_empty());
        set_locale("zh-CN");
        assert!(!t!("hello").is_empty());
    }
}
