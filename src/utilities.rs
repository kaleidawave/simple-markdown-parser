use super::{
    parse, parse_with_options, MarkdownElement, MarkdownParseError, MarkdownPart,
    MarkdownTextElement, ParseOptions, RawText, TextDecoration,
};

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

/// # Errors
/// errors from markdown parsing
#[allow(clippy::result_unit_err)]
pub fn parse_with_header_information<'a, T>(
    on: &'a str,
    options: ParseOptions,
    mut cb: impl for<'b> FnMut(&'b Vec<RawText<'a>>, MarkdownElement<'a>) -> Result<(), T>,
) -> Result<(), MarkdownParseError<T>> {
    let mut header_chain = Vec::new();
    parse_with_options(on, options, 0, |element| {
        if let MarkdownElement::Heading { level, content } = element {
            let raw_level = level as usize - 1;
            if header_chain.len() < raw_level {
                header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
            } else {
                let _ = header_chain.drain(raw_level..);
            }
            let result = cb(&header_chain, element);
            header_chain.push(content);
            result
        } else {
            cb(&header_chain, element)
        }
    })
}

/// # Errors
/// errors from markdown parsing
#[allow(clippy::result_unit_err)]
pub fn parse_sections<'a>(
    on: &'a str,
    mut cb: impl for<'b> FnMut(&'b Vec<RawText<'a>>, &'b [MarkdownElement<'a>]),
) -> Result<(), ()> {
    let mut header_chain = Vec::new();
    let mut inner = Vec::new();

    let result = parse::<()>(on, |element| {
        if let MarkdownElement::Heading { level, content } = element {
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
            header_chain.push(content);
        } else {
            inner.push(element);
        }
        Ok(())
    });

    if result.is_ok() {
        cb(&header_chain, &inner);
    }

    result.map_err(|_err| ())
}

#[derive(Default, Clone)]
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

pub fn parse_code_blocks(
    on: &str,
    mut cb: impl FnMut(CodeBlock),
) -> Result<(), MarkdownParseError<()>> {
    let mut header_chain: Vec<RawText> = Vec::new();
    let mut current_block = CodeBlock::default();
    // let mut blocks = on.split("\n").collect::<Vec<_>>();

    let result = parse(on, |element| {
        if let MarkdownElement::Heading { level, content } = element {
            let mut block = std::mem::take(&mut current_block);
            if !block.code.is_empty() {
                block.location = header_chain.iter().map(|link| link.0.to_owned()).collect();
                cb(block);
            }

            let raw_level = level as usize - 1;
            if header_chain.len() < raw_level {
                header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
            } else {
                let _ = header_chain.drain(raw_level..);
            }
            header_chain.push(content);
        } else if let MarkdownElement::CodeBlock(crate::CodeBlock { language, code }) = element {
            language.clone_into(&mut current_block.language);
            code.clone_into(&mut current_block.code);
        } else if let MarkdownElement::Paragraph(content) = element {
            current_block.information.push_str(content.0);
        } else if let MarkdownElement::Quote(content) = element {
            current_block.information.push_str(content.inner);
        } else if let MarkdownElement::ListItem { content, .. } = element {
            current_block.items.push(content.0.to_owned());
        }

        Ok(())
    });

    if !current_block.code.is_empty() {
        current_block.location = header_chain.iter().map(|link| link.0.to_owned()).collect();
        cb(current_block);
    }

    result
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
    let mut blocks: Vec<CodeBlock> = Vec::new();
    let _result = parse_code_blocks(on, |block| blocks.push(block));

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
        // TODO
        let options = crate::ParseOptions::default();
        let mut bytes: Vec<u8> = Vec::new();
        let _ = crate::extras::emit::markdown_to_html(
            &self.markdown_content,
            &mut bytes,
            emitter,
            options,
            0,
        );
        match String::from_utf8(bytes) {
            Ok(result) => result,
            Err(_) => String::from("Non Utf8 output or markdown parser error"),
        }
    }
}

/// Headings of level 1, 2 & 3 denote sections and slides. Levels 4, 5 & 6 denote inner headings
#[must_use]
#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn extract_slides(on: &str) -> Vec<Slide> {
    let mut header_chain: Vec<RawText> = Vec::new();
    let mut slides: Vec<Slide> = Vec::new();
    let mut current_slide = Slide::default();
    let mut start: usize = 0;

    let mut in_code_block = false;

    for line in on.lines() {
        if line.starts_with("```") {
            in_code_block ^= true;
        } else if in_code_block {
            continue;
        }
        let heading_level = if line.starts_with('#') {
            let level = line.chars().take_while(|c| *c == '#').count();
            // Fixes tags
            line[level..]
                .starts_with(char::is_whitespace)
                .then_some(level)
        } else {
            None
        };
        if let Some(level) = heading_level {
            if level <= 4 {
                let mut slide = std::mem::take(&mut current_slide);

                let end = line.as_ptr() as usize - on.as_ptr() as usize;
                let content = &on[start..end];

                if !content.trim().is_empty() {
                    content.clone_into(&mut slide.markdown_content);
                    slide.location = header_chain.iter().map(|link| link.0.to_owned()).collect();
                    slides.push(slide);
                }

                // TODO sub_ptr https://github.com/rust-lang/rust/issues/95892
                start = (line.as_ptr() as usize - on.as_ptr() as usize) + line.len();

                let raw_level = level - 1;
                if header_chain.len() < raw_level {
                    header_chain.extend((header_chain.len()..raw_level).map(|_| RawText("")));
                } else {
                    let _ = header_chain.drain(raw_level..);
                }
                header_chain.push(RawText(line[level..].trim()));
            }
        }
    }

    // Left over
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

        let _result = parse::<()>(on, |element| {
            if let Some(content) = element.inner_paragraph_raw() {
                analyser.paragraph(content);
            }

            if let Some(content) = element.parts_like() {
                for sentence in content.0.split('.') {
                    analyser.sentence(sentence);
                }

                for part in content.parts() {
                    // TODO links
                    if let crate::MarkdownPart::Plain = part.kind {
                        for word in part.on.split(&[' ', ',', '.', '!', '?']) {
                            let word = narrow_word(word);
                            if !word.is_empty() {
                                analyser.word(word);
                            }
                        }
                    }
                }
            } else {
                // Might be missing here
            }

            Ok(())
        });
    }
}

pub mod extraction {
    use crate::{parse, MarkdownElement, MarkdownParseError, MarkdownTextElement};

    #[derive(Clone, Copy)]
    pub enum Stop<'a> {
        FirstSection,
        MatchingLevel,
        AtHeader(&'a str),
    }

    fn find_first_new_line_offset(on: &str) -> usize {
        let mut chars = on.char_indices();
        while let Some((idx, chr)) = chars.next_back() {
            if let '\n' = chr {
                return on.len() - idx;
            }
        }
        on.len()
    }

    #[must_use]
    pub fn between_headers<'a>(
        on: &'a str,
        from_heading: Option<&str>,
        to: Option<Stop>,
    ) -> &'a str {
        if from_heading.is_none() && to.is_none() {
            return on;
        }

        let mut start: Option<usize> = from_heading.is_none().then_some(0);
        let mut matching_level = 0;

        let end = parse::<usize>(on, |element| {
            if let MarkdownElement::Heading { level, content } = element {
                if start.is_some() {
                    let content_offset = content.0.as_ptr() as usize;
                    let diff = content_offset - on.as_ptr() as usize;
                    let to_new_line = find_first_new_line_offset(&on[..diff]);
                    let end = diff - to_new_line;
                    match to.unwrap() {
                        Stop::FirstSection => {
                            return Err(end);
                        }
                        Stop::MatchingLevel => {
                            if level <= matching_level {
                                return Err(end);
                            }
                        }
                        Stop::AtHeader(header_end) => {
                            if content.0.eq_ignore_ascii_case(header_end) {
                                return Err(end);
                            }
                        }
                    }
                } else if content.0.eq_ignore_ascii_case(from_heading.unwrap()) {
                    let content_offset = content.0.as_ptr() as usize;
                    let diff = content_offset - on.as_ptr() as usize;
                    let to_new_line = find_first_new_line_offset(&on[..diff]);
                    start = Some(diff - to_new_line);
                    matching_level = level;
                    if to.is_none() {
                        return Err(on.len());
                    }
                }
            }
            Ok(())
        });

        if let Some(start) = start {
            if let Err(MarkdownParseError::FromCallback(end)) = end {
                &on[start..end]
            } else {
                &on[start..]
            }
        } else {
            ""
        }
    }

    pub fn links<'a>(on: &'a str, cb: impl Fn(&'a str, &'a str)) {
        parse::<usize>(on, |element| {
            if let MarkdownElement::ListItem { content, .. } = element {
                let parts = content.parts().collect::<Vec<_>>();
                if let &[MarkdownTextElement {
                    on,
                    kind: crate::MarkdownPart::ExternalLink { to },
                    decoration: _,
                }] = parts.as_slice()
                {
                    cb(on, to);
                }
            }
            Ok(())
        })
        .unwrap();
    }
}

pub(crate) fn strip_number_prefix(on: &str) -> Option<&str> {
    let level = on.chars().take_while(|c| matches!(*c, '1'..'9')).count();
    if 0 < level && level < 9 {
        on[level..].strip_prefix([')', '.'])
    } else {
        None
    }
}

pub(crate) fn strip_surrounds<'a>(on: &'a str, left: &str, right: &str) -> Option<&'a str> {
    on.trim()
        .strip_prefix(left)
        .and_then(|line| line.strip_suffix(right))
        .map(str::trim)
}

pub(crate) fn strip_upto_three_spaces(on: &str) -> &str {
    let level = on.chars().take_while(|c| matches!(*c, ' ')).count();
    if level <= 3 {
        &on[level..]
    } else {
        on
    }
}

pub(crate) fn strip_upto_one_new_line(on: &str) -> &str {
    if let Some(rest) = on.strip_prefix('\n') {
        strip_upto_three_spaces(rest)
    } else {
        on
    }
}

#[must_use]
pub fn parse_arguments(on: &str) -> Vec<(&str, &str)> {
    let mut arguments = Vec::new();
    let mut key: Option<&str> = None;
    let mut upto = 0;
    let mut in_string = false;

    for (idx, chr) in on.char_indices() {
        if let Some(current_key) = key {
            let value = on[upto..idx].trim();
            if let (' ', false, false) = (chr, in_string, value.is_empty()) {
                arguments.push((current_key, value));
                upto = idx;
                key = None;
            } else if let '"' = chr {
                in_string = !in_string;
            }
        } else if let '=' = chr {
            let key_acc = &on[upto..idx];
            key = Some(key_acc.trim());
            upto = idx + 1;
        }
    }

    if let Some(current_key) = key {
        if in_string {
            eprintln!("missing '\"'");
        }
        let value = on[upto..].trim();
        arguments.push((current_key, value));
    }

    if !on.is_empty() && arguments.is_empty() {
        arguments.push(("", on));
    }

    arguments
}

#[derive(Default, Clone, Copy)]
pub struct AsMarkdownOptions {
    pub uses_crlf: bool,
    pub skip_comments: bool,
}

impl<'a> MarkdownElement<'a> {
    #[must_use]
    pub fn as_markdown(&self, options: AsMarkdownOptions) -> String {
        let new_line = if options.uses_crlf { "\r\n" } else { "\n" };
        match self {
            Self::Heading { level, content } => {
                let mut s = "#".repeat(*level as usize);
                s.push(' ');
                s.push_str(content.0);
                s
            }
            Self::ListItem {
                level,
                content,
                enumerated: _,
                checked,
            } => {
                // TODO enumerated
                let mut s = "\t".repeat(*level as usize);
                s.push_str("- ");
                if let Some(checked) = checked {
                    s.push_str(if *checked { "[x]" } else { "[ ]" });
                }
                s.push_str(content.0);
                s
            }
            Self::CodeBlock(crate::CodeBlock { language, code }) => {
                let mut max = 3;
                for (idx, _) in code.match_indices("```") {
                    let backticks = code[idx..]
                        .find(|chr| chr != '`')
                        .map_or(code.len(), |idx| idx + 1);
                    max = std::cmp::max(max, backticks);
                }
                let indent = "`".repeat(max);
                format!("{indent}{language}{new_line}{code}{indent}")
            }
            Self::Paragraph(content) => content.0.to_owned(),
            Self::Quote(content) => content.inner.to_owned(),
            Self::Frontmatter(frontmatter) => {
                format!("---{new_line}{source}---", source = frontmatter.0)
            }
            // #[cfg(feature = "html")]
            // Self::HTMLElement { element: _, source } => source.to_string(),
            Self::Empty => String::new(),
            Self::CommentBlock(comment) => {
                if options.skip_comments {
                    String::default()
                } else {
                    format!("%%{new_line}{comment}{new_line}%%")
                }
            }
            item => format!("TODO {item:?}"),
        }
    }

    /// Paragraph content like elements
    #[must_use]
    pub fn inner_paragraph_raw(&self) -> Option<&str> {
        if let MarkdownElement::Paragraph(content) = self {
            Some(content.0)
        } else if let MarkdownElement::Quote(content) = self {
            // TODO these can be sometimes made up of elements
            Some(content.inner)
        } else {
            None
        }
    }

    #[must_use]
    pub fn parts_like(&self) -> Option<RawText<'a>> {
        if let MarkdownElement::Heading { content, .. }
        | MarkdownElement::Paragraph(content)
        | MarkdownElement::ListItem { content, .. } = self
        {
            Some(*content)
        } else {
            // no quote here
            None
        }
    }

    #[allow(clippy::match_same_arms, clippy::too_many_lines)]
    #[must_use]
    pub fn debug_with_options(&self, include_content: bool) -> String {
        use std::fmt::Write;

        fn from_parts(content: &RawText<'_>) -> String {
            let parts = content.parts().collect::<Vec<_>>();
            if let &[MarkdownTextElement {
                on,
                kind: MarkdownPart::Plain,
                decoration: TextDecoration::NONE,
            }] = parts.as_slice()
            {
                format!("{on:?}")
            } else {
                let mut s = "[".to_owned();
                for part in &parts {
                    if s.len() > 1 {
                        s.push_str(", ");
                    }
                    write!(&mut s, "{:?}", part.kind).unwrap();
                    if let Some('}') = s.chars().next_back() {
                        s.push(' ');
                    }
                    s.push('(');
                    write!(&mut s, "{:?}", part.on).unwrap();
                    if part.decoration != TextDecoration::NONE {
                        if part.decoration.contains(TextDecoration::BOLD) {
                            s.push_str(", bold");
                        }
                        if part.decoration.contains(TextDecoration::EMPHASIS) {
                            s.push_str(", emphasised");
                        }
                        if part.decoration.contains(TextDecoration::HIGHLIGHTED) {
                            s.push_str(", highlighted");
                        }
                        if part.decoration.contains(TextDecoration::STRIKETHROUGH) {
                            s.push_str(", strikethrough");
                        }
                        if part.decoration.contains(TextDecoration::SUPERSCRIPT) {
                            s.push_str(", superscript");
                        }
                        if part.decoration.contains(TextDecoration::SUBSCRIPT) {
                            s.push_str(", subscript");
                        }
                    }
                    s.push(')');
                }
                s.push(']');
                s
            }
        }

        match self {
            MarkdownElement::Heading { level, content } => {
                let inner = if include_content {
                    format!(", content: {} ", from_parts(content))
                } else {
                    String::new()
                };
                format!("Heading {{ level: {level}{inner}}}")
            }
            MarkdownElement::Paragraph(content) => {
                if include_content {
                    format!("Paragraph({})", from_parts(content))
                } else {
                    "Paragraph".to_owned()
                }
            }
            MarkdownElement::ListItem {
                level,
                content,
                enumerated,
                checked,
            } => {
                let inner = if include_content {
                    format!(", content: {} ", from_parts(content))
                } else {
                    String::new()
                };
                format!("ListItem {{ level: {level}, enumerated: {enumerated:?}, checked: {checked:?}{inner}}}")
            }
            MarkdownElement::CodeBlock(crate::CodeBlock { language, code }) => {
                if language.is_empty() {
                    format!("CodeBlock {{ code: {code:?} }}", code = code.trim())
                } else {
                    format!(
                        "CodeBlock {{ language: {language:?}, code: {code:?} }}",
                        code = code.trim()
                    )
                }
            }
            MarkdownElement::Quote(crate::QuoteBlock { alert, inner }) => {
                let inner = if include_content {
                    let mut s = "[".to_owned();
                    // FUTURE pass options down?
                    let options = crate::ParseOptions::default();
                    let _result = crate::parse_with_options::<()>(inner, options, 1, |item| {
                        if s.len() > 1 {
                            s.push_str(", ");
                        }
                        write!(&mut s, "{}", item.debug_with_options(include_content)).unwrap();
                        Ok(())
                    });
                    s.push(']');
                    s
                } else {
                    "...".to_string()
                };
                if let Some(alert) = alert {
                    format!("QuoteBlock {{ alert: {alert:?}, inner: {inner} }}")
                } else {
                    format!("QuoteBlock {{ inner: {inner} }}")
                }
            }
            MarkdownElement::Table(table) => {
                // TODO header?
                let mut source = String::from("[");
                for (idx, row) in table.rows().enumerate() {
                    if idx != 0 {
                        source.push_str(", ");
                    }
                    source.push('[');
                    for (idx, cell) in row.cells().enumerate() {
                        if idx != 0 {
                            source.push_str(", ");
                        }
                        source.push_str(&from_parts(&cell));
                    }
                    source.push(']');
                }
                source.push(']');
                format!("Table({source})")
            }
            MarkdownElement::CommandBlock(command_block) => {
                let inner = if include_content {
                    let mut s = "[".to_owned();
                    // FUTURE pass options down?
                    let options = crate::ParseOptions::default();
                    let _result = crate::parse_with_options::<()>(
                        command_block.inner.0,
                        options,
                        1,
                        |item| {
                            if s.len() > 1 {
                                s.push_str(", ");
                            }
                            write!(&mut s, "{}", item.debug_with_options(include_content)).unwrap();
                            Ok(())
                        },
                    );
                    s.push(']');
                    s
                } else {
                    "...".to_string()
                };
                format!(
                    "CommandBlock {{ name: {name}, arguments: {arguments:?}, inner: {inner} }}",
                    name = command_block.name,
                    arguments = command_block.parse_arguments()
                )
            }
            // #[cfg(feature = "html")]
            // MarkdownElement::HTMLElement { element, .. } => {
            //     format!("HTMLElement({element:?})")
            // }
            #[cfg(feature = "yaml")]
            MarkdownElement::Frontmatter(frontmatter) => {
                let mut s = "Frontmatter { ".to_owned();
                let _ = frontmatter.parse_yaml(|key, value| {
                    if s.len() > 14 {
                        s.push_str(", ");
                    }
                    write!(&mut s, "{key:?} -> {value:?}").unwrap();
                });
                s.push_str(" }");
                s
            }
            // rest
            item => format!("{item:?}"),
        }
    }
}

impl RawText<'_> {
    #[must_use]
    pub fn no_decoration(&self) -> String {
        let mut s = String::new();
        for part in crate::PartsIterator::new(self.0) {
            if let crate::MarkdownPart::Plain = part.kind {
                s.push_str(part.on);
            }
        }
        s
    }
}
