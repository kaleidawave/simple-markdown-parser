use super::{MarkdownElement, MarkdownPart, MarkdownTextElement, RawText, TextDecoration};

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
            Self::CodeBlock(crate::CodeBlock {
                language,
                code,
                indented_block: _,
            }) => {
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
            Self::Quote(content) => content.inner.0.to_owned(),
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
            Some(content.inner.0)
        } else {
            None
        }
    }

    #[must_use]
    pub fn parts_like(&self) -> Option<RawText<'a>> {
        if let MarkdownElement::Heading { content, .. } | MarkdownElement::Paragraph(content) = self
        {
            Some(*content)
        } else {
            // no quote or list item here
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
                    if part.on.is_empty() {
                        continue;
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
            MarkdownElement::List(list) => {
                let inner = if include_content {
                    let mut s = "[".to_owned();
                    // FUTURE pass options down?
                    let options = crate::ParseOptions::default();
                    list.parse_inner(|crate::ListItem { content, checked, enumerated  }| {
                        let mut inner = "[".to_owned();
                        // We can simplify the output if it is just a paragraph
                        let mut just_paragraph = None;
                        let _result = crate::parse_with_options::<()>(
                            content.0,
                            options,
                            content.1,
                            |item| {
                                if inner.len() > 1 {
                                    let _ = just_paragraph.take();
                                    inner.push_str(", ");
                                } else if let MarkdownElement::Paragraph(item) = item {
                                    just_paragraph = Some(item)
                                };
                                write!(&mut inner, "{}", item.debug_with_options(include_content))
                                    .unwrap();
                                Ok(())
                            },
                        );
                        if s.len() > 1 {
                            s.push_str(", ");
                        }
                        let inner = if let Some(just_paragraph) = just_paragraph {
                            from_parts(&just_paragraph)
                        } else {
                            inner.push(']');
                            inner
                        };
                        s.push_str(
                            &format!("ListItem {{ enumerated: {enumerated:?}, checked: {checked:?}, inner: {inner} }}")
                        );
                    });
                    s.push(']');
                    s
                } else {
                    "...".to_string()
                };
                format!("List({})", inner)
            }
            MarkdownElement::CodeBlock(crate::CodeBlock {
                language,
                code,
                indented_block,
            }) => {
                if language.is_empty() {
                    format!(
                        "CodeBlock {{ indented_block: {indented_block:?}, code: {code:?} }}",
                        code = code.trim()
                    )
                } else {
                    format!(
                        "CodeBlock {{ language: {language:?}, indented_block: {indented_block:?}, code: {code:?} }}",
                        code = code.trim()
                    )
                }
            }
            MarkdownElement::Quote(crate::QuoteBlock { alert, inner }) => {
                let inner = if include_content {
                    let mut s = "[".to_owned();
                    // FUTURE pass options down?
                    let options = crate::ParseOptions::default();
                    let temp = crate::ContainerResidue(">"); // Should be inner.1 or find last quote thing...?
                    let _result = crate::parse_with_options::<()>(inner.0, options, temp, |item| {
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
                        Default::default(),
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
        for part in crate::PartsIterator::new(self.0, self.1) {
            if let crate::MarkdownPart::Plain = part.kind {
                s.push_str(part.on);
            }
        }
        s
    }
}

impl<'a> crate::Table<'a> {
    pub fn rows(&self) -> impl Iterator<Item = TableRow<'a>> {
        let mut lines = self.0.lines();
        let header = lines.next().expect("no heading (empty table)");
        std::iter::once(TableRow(header)).chain(lines.skip(1).map(TableRow))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TableRow<'a>(pub(crate) &'a str);

impl<'a> TableRow<'a> {
    pub fn cells(&self) -> impl Iterator<Item = crate::RawText<'a>> {
        let inner = &self.0[1..(self.0.len() - 1)];
        inner
            .split('|')
            .map(|part| crate::RawText(part, Default::default()))
    }
}

impl<'a> crate::CommandBlock<'a> {
    #[must_use]
    #[allow(clippy::collapsible_else_if)]
    pub fn parse_arguments(&self) -> Vec<(&'a str, &'a str)> {
        let mut arguments = Vec::new();
        let mut key: Option<&str> = None;
        let mut upto = 0;
        let mut in_string = false;

        for (idx, chr) in self.arguments.char_indices() {
            if let Some(current_key) = key {
                let value = self.arguments[upto..idx].trim();
                if let (' ', false, false) = (chr, in_string, value.is_empty()) {
                    arguments.push((current_key, value));
                    upto = idx;
                    key = None;
                } else if let '"' = chr {
                    in_string = !in_string;
                }
            } else if let '=' = chr {
                let key_acc = &self.arguments[upto..idx];
                key = Some(key_acc.trim());
                upto = idx + 1;
            }
        }

        if let Some(current_key) = key {
            if in_string {
                eprintln!("missing '\"'");
            }
            let value = self.arguments[upto..].trim();
            arguments.push((current_key, value));
        }

        if !self.arguments.is_empty() && arguments.is_empty() {
            arguments.push(("", self.arguments));
        }

        arguments
    }
}

impl crate::Frontmatter<'_> {
    #[cfg(feature = "yaml")]
    pub fn parse_yaml(
        &self,
        cb: impl for<'b> FnMut(
            &'b [simple_yaml_parser::YAMLKey<'_>],
            simple_yaml_parser::RootYAMLValue<'_>,
        ),
    ) -> Result<(), simple_yaml_parser::YAMLParseError> {
        simple_yaml_parser::parse(self.0, cb)
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
    let level = on.find(|c| c != ' ').unwrap_or(on.len());
    if level <= 3 {
        &on[level..]
    } else {
        on
    }
}

pub(crate) fn strip_prefix_n_with_upto_three_spaces<'a>(
    mut on: &'a str,
    prefix: &str,
    n: usize,
) -> &'a str {
    for _ in 0..n {
        on = strip_upto_three_spaces(on);
        on = on.strip_prefix(prefix).unwrap_or(on);
    }
    on
}

pub(crate) fn starts_with_new_line_sequence(on: &str) -> bool {
    on.starts_with("\r\n") || on.starts_with("\n")
}

pub(crate) fn count_new_line_sequence(on: &str) -> usize {
    if on.starts_with("\r\n") {
        2
    } else if on.starts_with("\n") {
        1
    } else {
        panic!("string does not start with new line sequence")
    }
}

pub(crate) fn find_new_line_sequence(on: &str) -> Option<(usize, usize)> {
    for (idx, matched) in on.match_indices(['\r', '\n']) {
        // TODO does this check need to be done?
        if matched == "\r" && on[idx..].starts_with("\r\n") {
            return Some((idx, 2));
        } else {
            return Some((idx, 1));
        }
    }
    None
}

pub(crate) fn html_balanced(on: &str, tag_name: &str) -> bool {
    // FUTURE needs improving
    let mut depth: u8 = 0;
    for (idx, _) in on.match_indices("<") {
        let rest = &on[idx..];
        let (offset, new_depth) = if rest.starts_with("</") {
            (2, depth.saturating_sub(1))
        } else {
            (1, depth + 1)
        };
        let out = rest
            .get(offset..)
            .unwrap_or_default()
            .split_once(|chr: char| !chr.is_alphanumeric())
            .map_or(rest, |(l, _)| l);
        if out == tag_name {
            depth = new_depth;
        }
    }
    depth == 0
}

pub struct EdibleLines<'a> {
    start: usize,
    last: usize,
    on: &'a str,
}

impl<'a> EdibleLines<'a> {
    pub fn new(on: &'a str) -> Self {
        EdibleLines {
            on,
            start: 0,
            last: 0,
        }
    }

    pub fn moving_on(&mut self) {
        self.start = self.last;
    }

    pub fn peek_line(&self) -> Option<&'a str> {
        self.on
            .get(self.last..)
            .map(|rest| rest.lines().next().unwrap_or(rest))
    }

    pub fn on(&self) -> &'a str {
        self.on
    }

    pub fn is_at_start(&self) -> bool {
        self.start == 0
    }

    pub fn skip_next(&mut self) {
        if let Some((next, len)) = find_new_line_sequence(&self.on[self.last..]) {
            self.last += next + len;
        } else {
            self.last = self.on.len();
        }
    }
}

impl<'a> Iterator for EdibleLines<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((next, len)) = find_new_line_sequence(&self.on[self.last..]) {
            let slice = &self.on[self.start..(self.last + next)];
            self.last += next + len;
            Some(slice)
        } else if self.last < self.on.len() {
            self.last = self.on.len();
            Some(&self.on[self.start..])
        } else {
            None
        }
    }
}