use super::{parse, MarkdownElement, RawText};

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

/// # Errors
/// errors from markdown parsing
#[allow(clippy::result_unit_err)]
pub fn parse_with_header_information<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b Vec<RawText<'a>>, MarkdownElement<'a>),
) -> Result<(), ()> {
    let mut header_chain = Vec::new();
    parse(on, |element| {
        if let MarkdownElement::Heading { level, text } = element {
            let raw_level = level as usize - 1;
            if header_chain.len() < raw_level {
                header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
            } else {
                let _ = header_chain.drain(raw_level..);
            }
            cb(&header_chain, element);
            header_chain.push(text);
        } else {
            cb(&header_chain, element);
        }
    })
}

/// # Errors
/// errors from markdown parsing
#[allow(clippy::result_unit_err)]
pub fn parse_blocks<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b Vec<RawText<'a>>, &'b [MarkdownElement<'a>]),
) -> Result<(), ()> {
    let mut header_chain = Vec::new();
    let mut inner = Vec::new();

    let result = parse(on, |element| {
        if let MarkdownElement::Heading { level, text } = element {
            // Run when next one begins
            {
                cb(&header_chain, &inner);
                let _ = inner.drain(..);
            }

            let raw_level = level as usize - 1;
            if header_chain.len() < raw_level {
                header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
            } else {
                let _ = header_chain.drain(raw_level..);
            }
            header_chain.push(text);
        } else {
            inner.push(element);
        }
    });

    if result.is_ok() {
        cb(&header_chain, &inner);
    }

    result
}

#[derive(Default)]
#[cfg_attr(
    target_family = "wasm",
    derive(tsify::Tsify, serde::Serialize),
    tsify(into_wasm_abi)
)]
pub struct CodeBlock {
    location: Vec<String>,
    language: String,
    code: String,
    /// From quotes and content
    information: String,
    /// From list items
    items: Vec<String>,
}

#[cfg(target_family = "wasm")]
#[derive(tsify::Tsify, serde::Serialize)]
#[tsify(into_wasm_abi)]
pub struct VecCodeBlock(Vec<CodeBlock>);

#[cfg(target_family = "wasm")]
impl From<Vec<CodeBlock>> for VecCodeBlock {
    fn from(blocks: Vec<CodeBlock>) -> Self {
        Self(blocks)
    }
}

#[cfg(not(target_family = "wasm"))]
pub type VecCodeBlock = Vec<CodeBlock>;

#[must_use]
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn extract_code_blocks(on: &str) -> VecCodeBlock {
    let mut header_chain: Vec<RawText> = Vec::new();
    let mut blocks: Vec<CodeBlock> = Vec::new();
    let mut current_block = CodeBlock::default();
    // let mut blocks = on.split("\n").collect::<Vec<_>>();

    let _result = parse(on, |element| {
        if let MarkdownElement::Heading { level, text } = element {
            let mut block = std::mem::take(&mut current_block);
            if !block.code.is_empty() {
                block.location = header_chain.iter().map(|link| link.0.to_owned()).collect();
                blocks.push(block);
            }

            let raw_level = level as usize - 1;
            if header_chain.len() < raw_level {
                header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
            } else {
                let _ = header_chain.drain(raw_level..);
            }
            header_chain.push(text);
        } else if let MarkdownElement::CodeBlock { language, code } = element {
            language.clone_into(&mut current_block.language);
            code.clone_into(&mut current_block.code);
        } else if let MarkdownElement::Paragraph(content) = element {
            current_block.information.push_str(content.0);
        } else if let MarkdownElement::Quote(content) = element {
            current_block.information.push_str(content.0);
        } else if let MarkdownElement::ListItem { level: _, text } = element {
            current_block.items.push(text.0.to_owned());
        }
    });

    if !current_block.code.is_empty() {
        blocks.push(current_block);
    }

    // .into for WASM fix
    #[allow(clippy::useless_conversion)]
    blocks.into()
}

// Unfortuantly the same definition bc of <https://users.rust-lang.org/t/cant-use-cfg-attr-with-wasm-bindgen-skip/112072>
#[cfg(target_family = "wasm")]
#[wasm_bindgen]
#[derive(Default, Debug)]
pub struct Slide {
    location: Vec<String>,
    markdown_content: String,
}

#[cfg(not(target_family = "wasm"))]
#[derive(Default, Debug)]
pub struct Slide {
    pub location: Vec<String>,
    pub markdown_content: String,
}

#[cfg_attr(target_family = "wasm", wasm_bindgen)]
impl Slide {
    #[must_use]
    #[cfg_attr(target_family = "wasm", wasm_bindgen(getter))]
    pub fn location(&self) -> Vec<String> {
        self.location.clone()
    }

    #[must_use]
    #[cfg_attr(target_family = "wasm", wasm_bindgen(getter))]
    pub fn markdown_content(&self) -> String {
        self.markdown_content.clone()
    }

    #[must_use]
    #[cfg(target_family = "wasm")]
    #[wasm_bindgen]
    pub fn to_html(&self, emitter: Option<crate::extras::emit::FeatureEmitterWASM>) -> String {
        crate::extras::emit::markdown_to_html_string(&self.markdown_content, emitter)
    }

    #[must_use]
    #[cfg(not(target_family = "wasm"))]
    pub fn to_html(&self, emitter: &mut impl crate::extras::emit::FeatureEmitter) -> String {
        let mut bytes: Vec<u8> = Vec::new();
        let _ = crate::extras::emit::markdown_to_html(&self.markdown_content, &mut bytes, emitter);
        match String::from_utf8(bytes) {
            Ok(result) => result,
            Err(_) => String::from("Non Utf8 output or markdown parser error"),
        }
    }
}

#[must_use]
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn extract_slides(on: &str) -> Vec<Slide> {
    let mut header_chain: Vec<RawText> = Vec::new();
    let mut slides: Vec<Slide> = Vec::new();
    let mut current_slide = Slide::default();
    let mut start: usize = 0;

    let _result = parse(on, |element| {
        if let MarkdownElement::Heading { level, text } = element {
            if level < 3 {
                let mut slide = std::mem::take(&mut current_slide);
                let end = text.0.as_ptr() as usize - on.as_ptr() as usize;
                let content = &on[(start + level as usize)..end];
                if !content.trim().is_empty() {
                    content.clone_into(&mut current_slide.markdown_content);
                    slide.location = header_chain.iter().map(|link| link.0.to_owned()).collect();
                    slides.push(slide);
                }
                // TODO sub_ptr https://github.com/rust-lang/rust/issues/95892
                start = (text.0.as_ptr() as usize - on.as_ptr() as usize) + text.0.len();
            }

            let raw_level = level as usize - 1;
            if header_chain.len() < raw_level {
                header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
            } else {
                let _ = header_chain.drain(raw_level..);
            }
            header_chain.push(text);
        }
    });

    {
        let content = &on[start..];
        if !content.trim().is_empty() {
            content.clone_into(&mut current_slide.markdown_content);
            current_slide.location = header_chain.iter().map(|link| link.0.to_owned()).collect();
            slides.push(current_slide);
        }
    }

    slides
}

pub mod lexical_analysis {
    use super::parse;

    pub trait LexicalAnalyser {
        fn word(&mut self, word: &str);

        /// **WARNING** called with decoration
        fn sentence(&mut self, sentence: &str);

        /// **WARNING** called with decoration
        fn paragraph(&mut self, paragraph: &str);
    }

    pub fn lexical_analysis(on: &str, analyser: &mut impl LexicalAnalyser) {
        fn narrow_word(word: &str) -> &str {
            let word = word.trim();
            let word = word.strip_prefix('(').unwrap_or(word);
            let word = word.strip_suffix('.').unwrap_or(word);
            let word = word.strip_suffix(',').unwrap_or(word);
            let word = word.strip_suffix(')').unwrap_or(word);
            word
        }

        let _result = parse(on, |element| {
            if let Some(text) = element.inner_paragraph_raw() {
                analyser.paragraph(text);
            }

            if let Some(text) = element.parts_like() {
                for sentence in text.0.split('.') {
                    analyser.sentence(sentence);
                }

                for part in text.parts() {
                    for word in part.no_decoration().split(&[' ', ',', '.', '!', '?']) {
                        let word = narrow_word(word);
                        if !word.is_empty() {
                            analyser.word(word);
                        }
                    }
                }
            } else {
                // Might be missing here
            }
        });
    }
}
