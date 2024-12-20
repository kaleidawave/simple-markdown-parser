#![doc = include_str!("../README.md")]

pub mod utilities;

/// Markdown block element
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MarkdownElement<'a> {
    Heading {
        level: u8,
        text: RawText<'a>,
    },
    Quote(RawText<'a>),
    Paragraph(RawText<'a>),
    ListItem {
        level: u8,
        text: RawText<'a>,
    },
    // TODO
    Table(Table<'a>),
    // TODO modifiers
    CodeBlock {
        language: &'a str,
        content: &'a str,
    },
    HTMLComment(&'a str),
    // TODO how much to do here
    HTMLElement(&'a str),
    // TODO at start?
    Frontmatter(&'a str),
    HorizontalRule,
    Media {
        alt: &'a str,
        link: Option<&'a str>,
        source: &'a str,
    },
    Footnote,
    Empty,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Table<'a>(pub(crate) &'a str);

impl<'a> Table<'a> {
    pub fn rows(&self) -> impl Iterator<Item = TableRow<'a>> {
        let mut lines = self.0.lines();
        let header = lines.next().unwrap();
        std::iter::once(TableRow(header)).chain(lines.skip(1).map(TableRow))
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TableRow<'a>(pub(crate) &'a str);

impl<'a> TableRow<'a> {
    pub fn cells(&self) -> impl Iterator<Item = RawText<'a>> {
        let inner = &self.0[1..(self.0.len() - 1)];
        inner.split('|').map(RawText)
    }
}

impl MarkdownElement<'_> {
    #[must_use]
    pub fn as_markdown(&self) -> String {
        match self {
            Self::Heading { level, text } => {
                let mut s = "#".repeat(*level as usize);
                s.push_str(text.0);
                s.push(' ');
                s
            }
            Self::ListItem { level, text } => {
                let mut s = "\t".repeat(*level as usize);
                s.push_str("- ");
                s.push_str(text.0);
                s
            }
            Self::CodeBlock { language, content } => {
                format!("```{language}\n{content}```")
                // let mut s = "```".to_owned();
                // s.push_str(language);
                // s.push_str("\n");
                // s.push_str("```");
                // s
            }
            Self::Paragraph(text) => text.0.to_owned(),
            Self::Quote(text) => {
                format!("> {text}", text = text.0)
            }
            Self::Empty => String::new(),
            item => format!("TODO {item:?}"),
        }
    }
}

/// (unsplit) Text inside markdown item
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawText<'a>(pub &'a str);

impl<'a> RawText<'a> {
    #[must_use]
    pub fn parts(&self) -> PartsIterator<'a> {
        PartsIterator::new(self.0)
    }

    #[must_use]
    pub fn no_decoration(&self) -> String {
        let mut s = String::new();
        for part in PartsIterator::new(self.0) {
            s.push_str(part.no_decoration());
        }
        s
    }
}

/// Some are prefixes, some are wrapped
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MarkdownTextElement<'a> {
    Plain(&'a str),
    // **hi**
    Bold(&'a str),
    /// __hi__
    Italic(&'a str),
    /// **__hi__**
    BoldAndItalic(&'a str),
    /// `code`
    Code(&'a str),
    /// ~~gone~~
    StrikeThrough(&'a str),
    /// :emoji:
    Emoji(&'a str),
    /// $\sin$
    Latex(&'a str),
    /// TODO WIP
    Expression(&'a str),

    Highlight(&'a str),
    Subscript(&'a str),
    Superscript(&'a str),
    /// #item
    Tag(&'a str),
    Link {
        /// TODO not great but..
        on: RawText<'a>,
        to: &'a str,
    },
    Media {
        alt: &'a str,
        source: &'a str,
    },
}

impl<'a> MarkdownTextElement<'a> {
    #[must_use]
    pub fn no_decoration(&self) -> &'a str {
        match self {
            MarkdownTextElement::Plain(i)
            | MarkdownTextElement::Bold(i)
            | MarkdownTextElement::Italic(i)
            | MarkdownTextElement::BoldAndItalic(i)
            | MarkdownTextElement::Code(i)
            | MarkdownTextElement::StrikeThrough(i)
            | MarkdownTextElement::Emoji(i)
            | MarkdownTextElement::Latex(i)
            | MarkdownTextElement::Highlight(i)
            | MarkdownTextElement::Subscript(i)
            | MarkdownTextElement::Superscript(i)
            | MarkdownTextElement::Tag(i) => i,
            MarkdownTextElement::Expression(_) | MarkdownTextElement::Media { .. } => "",
            MarkdownTextElement::Link { on: _, to: _ } => {
                eprintln!("TODO");
                ""
            }
        }
    }
}

// TODO want to do in main loop
#[allow(clippy::needless_lifetimes)]
fn decide<'a>(item: &'a str) -> MarkdownElement<'a> {
    let item = item.trim();
    if item.starts_with('#') {
        let level = item.chars().take_while(|c| *c == '#').count();
        MarkdownElement::Heading {
            level: level.try_into().expect("deep header"),
            text: RawText(item[level..].trim()),
        }
    } else if let Some(item) = item.strip_prefix('>') {
        MarkdownElement::Quote(RawText(item))
    } else if let "---" = item {
        MarkdownElement::HorizontalRule
    } else if let Some(item) = item.trim_start().strip_prefix('-') {
        // TODO one or the other
        let level = item.chars().take_while(|c| *c == '\t' || *c == ' ').count();
        MarkdownElement::ListItem {
            level: level.try_into().expect("deep list item"),
            text: RawText(item.trim()),
        }
    } else if item.is_empty() {
        MarkdownElement::Empty
    } else {
        MarkdownElement::Paragraph(RawText(item))
    }
}

#[derive(Default, Copy, Clone)]
pub struct ParseOptions {
    include_new_lines: bool,
}

pub fn parse<'a>(on: &'a str, cb: impl FnMut(MarkdownElement<'a>)) -> Result<(), ()> {
    parse_with_options(on, &ParseOptions::default(), cb)
}

/// Parse source using callback
#[allow(clippy::result_unit_err)]
pub fn parse_with_options<'a>(
    on: &'a str,
    options: &ParseOptions,
    mut cb: impl FnMut(MarkdownElement<'a>),
) -> Result<(), ()> {
    let mut since_new_line = 0;
    let mut start = 0;
    // Some => in_code
    let mut current_code_language = None;

    let mut in_frontmatter = false;
    let mut in_table = false;

    for (idx, chr) in on.char_indices() {
        if let '\n' = chr {
            let line = &on[since_new_line..idx];

            if current_code_language.is_some() {
                if let "```" = line.trim() {
                    cb(MarkdownElement::CodeBlock {
                        language: current_code_language.take().unwrap(),
                        content: &on[start..since_new_line],
                    });
                }
                since_new_line = idx + 1;
                continue;
            }

            let is_horizontal_rule = "---" == line.trim();

            if in_frontmatter {
                if is_horizontal_rule {
                    cb(MarkdownElement::Frontmatter(&on[start..since_new_line]));
                    in_frontmatter = false;
                }
                since_new_line = idx + 1;
                continue;
            }

            if in_table {
                if !line.ends_with('|') {
                    cb(MarkdownElement::Table(Table(&on[start..since_new_line])));
                    in_table = false;
                }
                since_new_line = idx + 1;
                continue;
            }

            since_new_line = idx + 1;

            if let Some(rest) = line.strip_prefix("```") {
                // TODO other motifiers here
                let language = rest.trim_end();
                current_code_language = Some(language);
            } else if line.starts_with('|') {
                in_table = true;
                continue;
            } else if start == 0 && is_horizontal_rule {
                in_frontmatter = true;
            } else {
                let result = decide(line);
                let to_add = !matches!(
                    (options.include_new_lines, result),
                    (false, MarkdownElement::Empty)
                );
                if to_add {
                    cb(result);
                }
            }

            start = since_new_line;
        }
    }

    if current_code_language.is_some() {
        todo!("error here");
    }

    let line = &on[start..];

    let result = decide(line);
    let to_add = !matches!(
        (options.include_new_lines, result),
        (false, MarkdownElement::Empty)
    );
    if to_add {
        cb(result);
    }

    Ok(())
}

/// Work in progress abstraction for iterating over markdown text sections giving decoration (bold, links, etc) information
/// TODO state
pub struct PartsIterator<'a> {
    on: &'a str,
    last: usize,
    in_tag: bool,
    pub in_bold: bool,
    pub in_italic: bool,
    in_code: bool,
    in_latex: bool,
    in_emoji: bool,
    in_link: bool,
    in_media: bool,
    in_expression: bool,
}

impl<'a> PartsIterator<'a> {
    #[must_use]
    pub fn new(on: &'a str) -> Self {
        Self {
            on,
            last: 0,
            in_tag: false,
            in_bold: false,
            in_italic: false,
            in_emoji: false,
            in_code: false,
            in_latex: false,
            in_link: false,
            in_media: false,
            in_expression: false,
        }
    }
}

impl<'a> Iterator for PartsIterator<'a> {
    type Item = MarkdownTextElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last >= self.on.len() {
            None
        } else {
            let mut link_text_end: Option<usize> = None;
            let mut bracket_depth: usize = 0;

            let mut range = &self.on[self.last..];
            let mut iterator = range.char_indices();

            while let Some((idx, chr)) = iterator.next() {
                if self.in_link || self.in_media {
                    if let Some(link_text_end) = link_text_end {
                        if idx == link_text_end + 1 {
                            if chr != '(' {
                                if self.in_link {
                                    self.last += idx;
                                    self.in_link = false;
                                    return Some(MarkdownTextElement::Link {
                                        on: RawText(&range[..link_text_end]),
                                        to: "",
                                    });
                                }
                                panic!("media parsing broken {chr}");
                            }
                        } else if let ')' = chr {
                            let in_brackets = &range[..link_text_end];
                            let in_parenthesis = &range[link_text_end + "](".len()..idx];
                            let element = if self.in_link {
                                self.in_link = false;
                                MarkdownTextElement::Link {
                                    on: RawText(in_brackets),
                                    to: in_parenthesis,
                                }
                            } else {
                                self.in_media = false;
                                MarkdownTextElement::Media {
                                    alt: in_brackets,
                                    source: in_parenthesis,
                                }
                            };

                            self.last += idx + 1;
                            return Some(element);
                        }
                    } else if let ']' = chr {
                        if let Some(reduced_depth) = bracket_depth.checked_sub(1) {
                            bracket_depth = reduced_depth;
                        } else {
                            link_text_end = Some(idx);
                        }
                    } else if let '[' = chr {
                        bracket_depth += 1;
                    }

                    continue;
                }

                // TODO escaped stuff etc
                if self.in_code {
                    if let '`' = chr {
                        self.last += idx + 1;
                        self.in_code = false;
                        return Some(MarkdownTextElement::Code(&range[..idx]));
                    }
                    continue;
                }
                // TODO escaped stuff etc
                if let (true, '$') = (self.in_latex, chr) {
                    self.last += idx + 1;
                    self.in_latex = false;
                    return Some(MarkdownTextElement::Latex(&range[..idx]));
                }
                // TODO escaped stuff etc
                if let (true, ':') = (self.in_emoji, chr) {
                    self.last += idx + 1;
                    self.in_emoji = false;
                    return Some(MarkdownTextElement::Emoji(&range[..idx]));
                }
                // TODO escaped stuff etc
                if let (true, '}') = (self.in_expression, chr) {
                    self.last += idx + 1;
                    self.in_expression = false;
                    return Some(MarkdownTextElement::Expression(&range[..idx]));
                }

                if self.in_tag && chr.is_whitespace() {
                    self.last += idx + 1;
                    self.in_tag = false;
                    return Some(MarkdownTextElement::Tag(&range[..idx]));
                }

                macro_rules! yield_current {
                    () => {{
                        let item = &range[..idx];
                        if !item.is_empty() {
                            return Some(MarkdownTextElement::Plain(item));
                        }
                        // Reset
                        range = &self.on[self.last..];
                        iterator = range.char_indices();
                    }};
                }

                match chr {
                    '`' => {
                        self.last += idx + 1;
                        self.in_code = true;
                        yield_current!();
                    }
                    '$' => {
                        self.last += idx + 1;
                        self.in_latex = true;
                        yield_current!();
                    }
                    '{' => {
                        self.last += idx + 1;
                        self.in_expression = true;
                        yield_current!();
                    }
                    ':' if range[(idx + 1)..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric) =>
                    {
                        // TODO check next is not whitespace etc
                        self.last += idx + 1;
                        self.in_emoji = true;
                        yield_current!();
                    }
                    '#' => {
                        self.last += idx + 1;
                        self.in_tag = true;
                        yield_current!();
                    }
                    '!' if range[idx..].starts_with("![") => {
                        self.last += idx + "![".len();
                        self.in_media = true;
                        yield_current!();
                    }
                    '[' => {
                        self.last += idx + '['.len_utf8();
                        self.in_link = true;
                        yield_current!();
                    }
                    '*' if range[idx..].starts_with("**") => {
                        self.last += idx + 2;
                        self.in_bold = !self.in_bold;
                        if self.in_bold {
                            yield_current!();
                        } else {
                            return Some(MarkdownTextElement::Bold(&range[..idx]));
                        }
                    }
                    '*' => {
                        self.last += idx + 1;
                        self.in_italic = !self.in_italic;
                        if self.in_italic {
                            yield_current!();
                        } else {
                            return Some(MarkdownTextElement::Italic(&range[..idx]));
                        }
                    }
                    _ => {}
                }
            }

            self.last = self.on.len();
            if range.is_empty() {
                None
            } else {
                // TODO errors left overs. But also others such as tags etc
                Some(MarkdownTextElement::Plain(range))
            }
        }
    }
}
