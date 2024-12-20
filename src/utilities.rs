use super::{parse, MarkdownElement, RawText};

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

pub trait LexicalAnalyser {
    fn word(&mut self, word: &str);

    /// **WARNING** called with decoration
    fn sentence(&mut self, sentence: &str);

    /// **WARNING** called with decoration
    fn paragraph(&mut self, paragraph: &str);
}

#[allow(clippy::result_unit_err)]
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
        if let MarkdownElement::Quote(text) | MarkdownElement::Paragraph(text) = element {
            analyser.paragraph(text.0);
        }

        if let MarkdownElement::Heading { text, .. }
        | MarkdownElement::Quote(text)
        | MarkdownElement::Paragraph(text)
        | MarkdownElement::ListItem { level: _, text } = element
        {
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
