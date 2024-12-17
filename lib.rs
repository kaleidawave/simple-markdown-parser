#![doc = include_str!("./README.md")]

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
    // TOOD
    Table {},
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

impl MarkdownElement<'_> {
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

impl RawText<'_> {
    pub fn parts(&self) -> PartsIterator<'_> {
        PartsIterator::new(self.0)
    }

    pub fn no_modifiers(&self) -> String {
        let mut s = String::new();
        for part in PartsIterator::new(self.0) {
            match part {
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
                | MarkdownTextElement::Tag(i) => s.push_str(i),
                MarkdownTextElement::Link { on, to: _ } => s.push_str(&on.no_modifiers()),
            }
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
}

// TODO want to do in main loop
#[allow(clippy::needless_lifetimes)]
fn decide<'a>(item: &'a str) -> MarkdownElement<'a> {
    let item = item.trim();
    if item.starts_with("#") {
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

/// Parse source using callback
#[allow(clippy::result_unit_err)]
pub fn parse<'a>(on: &'a str, mut cb: impl FnMut(MarkdownElement<'a>)) -> Result<(), ()> {
    let mut since_new_line = 0;
    let mut start = 0;
    // Some => in_code
    let mut current_code_language = None;
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

            since_new_line = idx + 1;

            if let Some(rest) = line.strip_prefix("```") {
                // TODO other motifiers here
                let language = rest.trim_end();
                current_code_language = Some(language);
            } else if line.starts_with("|") {
                todo!("table")
            } else {
                let result = decide(line);
                let to_add = !matches!(result, MarkdownElement::Empty);
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
    let to_add = !matches!(result, MarkdownElement::Empty);
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
}

impl<'a> PartsIterator<'a> {
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
        }
    }
}

impl<'a> Iterator for PartsIterator<'a> {
    type Item = MarkdownTextElement<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.on.len() == self.last {
            None
        } else {
            let range = &self.on[self.last..];
            let mut link_text_end: Option<usize> = None;

            for (idx, chr) in range.char_indices() {
                if self.in_link {
                    if let Some(link_text_end) = link_text_end {
                        if idx == link_text_end + 1 {
                            if chr != '(' {
                                self.last += idx;
                                self.in_link = false;
                                return Some(MarkdownTextElement::Link {
                                    on: RawText(&range[..link_text_end]),
                                    to: "",
                                });
                            }
                        } else if let ')' = chr {
                            self.last += idx + 1;
                            self.in_link = false;
                            return Some(MarkdownTextElement::Link {
                                on: RawText(&range[..link_text_end]),
                                to: &range[link_text_end + "](".len()..idx],
                            });
                        }
                    } else if let ']' = chr {
                        link_text_end = Some(idx);
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

                if self.in_tag && chr.is_whitespace() {
                    self.last += idx + 1;
                    self.in_tag = false;
                    return Some(MarkdownTextElement::Tag(&range[..idx]));
                }

                match chr {
                    '`' | '$' => {
                        self.last += idx + 1;
                        match chr {
                            '`' => {
                                self.in_code = true;
                            }
                            '$' => {
                                self.in_latex = true;
                            }
                            chr => unreachable!("{chr:?}"),
                        }
                        return Some(MarkdownTextElement::Plain(&range[..idx]));
                    }
                    ':' => {
                        // TODO check next is not whitespace etc
                        self.last += idx + 1;
                        self.in_emoji = true;
                        return Some(MarkdownTextElement::Plain(&range[..idx]));
                    }
                    '#' => {
                        self.last += idx + 1;
                        self.in_tag = true;
                        return Some(MarkdownTextElement::Plain(&range[..idx]));
                    }
                    '*' => {
                        // dbg!(&range[idx..]);
                        // TODO not quite right. also _
                        let to_return = &range[..idx];
                        let returned = if range[idx..].starts_with("**") {
                            self.last += idx + 2;
                            let existing = self.in_bold;
                            self.in_bold = !existing;
                            if existing {
                                MarkdownTextElement::Bold(to_return)
                            } else {
                                MarkdownTextElement::Plain(to_return)
                            }
                        } else {
                            self.last += idx + 1;
                            let existing = self.in_italic;
                            self.in_italic = !existing;
                            if existing {
                                MarkdownTextElement::Italic(to_return)
                            } else {
                                MarkdownTextElement::Plain(to_return)
                            }
                        };
                        return Some(returned);
                    }
                    '[' => {
                        self.last += idx + 1;
                        self.in_link = true;
                        return Some(MarkdownTextElement::Plain(&range[..idx]));
                    }
                    _ => {}
                }
            }
            self.last = self.on.len();
            // TODO left overs, tags etc
            Some(MarkdownTextElement::Plain(range))
        }
    }
}

/// Additional utilities for parsing markdown
pub mod utilities {
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
}
