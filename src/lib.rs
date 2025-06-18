#![doc = include_str!("../README.md")]

pub mod emit;
pub mod utilities;

/// Markdown block element
#[derive(Debug, Copy, Clone)]
pub enum MarkdownElement<'a> {
    Heading {
        level: u8,
        content: RawText<'a>,
    },
    Quote(QuoteBlock<'a>),
    Paragraph(RawText<'a>),
    List(List<'a>),
    Table(Table<'a>),
    // TODO modifiers
    CodeBlock(CodeBlock<'a>),
    BlockMathematics(BlockMathematics<'a>),
    CommandBlock(CommandBlock<'a>),
    /// Inside `%%` (from Obsidan)
    CommentBlock(&'a str),
    /// Includes HTML comments
    HTMLElement(HTMLElement<'a>),
    // TODO at start?
    Frontmatter(Frontmatter<'a>),
    HorizontalRule,
    // Footnote,
    Empty,
}

/// (unsplit) Text inside markdown item
#[derive(Debug, Copy, Clone)]
pub struct RawText<'a>(pub &'a str, pub ContainerResidue<'a>);

impl<'a> RawText<'a> {
    #[must_use]
    pub fn parts(&self) -> PartsIterator<'a> {
        PartsIterator::new(self.0, self.1)
    }
}

/// Extras (that IMO should be all false )
#[derive(Default, Copy, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ParseOptions {
    pub allow_asterisk_and_plus_as_list_prefixes: bool,
    pub heading_underscores: bool,
    pub record_empty_lines: bool,
    pub indented_code_blocks: bool,
    pub tilda_code_blocks: bool,
}

#[derive(Debug)]
pub enum MarkdownParseError<T> {
    FromCallback(T),
}

impl<T: std::fmt::Debug> std::fmt::Display for MarkdownParseError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        std::fmt::Debug::fmt(self, f)
    }
}

impl<T: std::fmt::Debug + std::fmt::Display> std::error::Error for MarkdownParseError<T> {}

/// # Errors
/// errors for unclosed blocks
pub fn parse<'a, T>(
    on: &'a str,
    cb: impl FnMut(MarkdownElement<'a>) -> Result<(), T>,
) -> Result<(), MarkdownParseError<T>> {
    parse_with_options(on, ParseOptions::default(), ContainerResidue::default(), cb)
}

pub fn parse_with_options<'a, T>(
    on: &'a str,
    options: ParseOptions,
    container_residue: ContainerResidue<'a>,
    cb: impl FnMut(MarkdownElement<'a>) -> Result<(), T>,
) -> Result<(), MarkdownParseError<T>> {
    MarkdownParser::parse_with_options(on, options, container_residue, cb)
}

#[derive(Clone, Copy, Debug)]
enum State<'a> {
    None,
    CodeBlock {
        language: &'a str,
        delimeter: &'a str,
    },
    Command {
        name: &'a str,
        arguments: &'a str,
    },
    Frontmatter,
    Table,
    MathematicsBlock,
    MarkdownComment,
    CommandHeader,
    Quote {
        alert: Option<&'a str>,
    },
    List,
    HTMLElement {
        tag_name: &'a str,
    },
}

// #[derive(Clone, Copy, Debug, Default)]
// pub enum ContainerResidue<'a> {
//     InQuote(utilities::QuoteDepth),
//     InList(utilities::ListDepth),
//     #[default]
//     None,
// }

// type ContainerResidue<'a> = ContainerResidue;

// impl ContainerResidue<'a> {
//     pub fn remove_parent_slice(self, line: &str) -> &str {
//         match self {
//             ContainerResidue<'a>::InQuote(quote_depth) => quote_depth.strip_chevrons(line),
//             ContainerResidue<'a>::InList(depth) => depth.strip_indent(line),
//             ContainerResidue::default() => line,
//         }
//     }
// }

pub struct MarkdownParser<'a, F> {
    reader: utilities::EdibleLines<'a>,
    state: State<'a>,
    container_residue: ContainerResidue<'a>,
    options: ParseOptions,
    cb: F,
}

pub const BASIC_LIST_PREFIXES: &[char] = &['-'];
pub const EXTENDED_LIST_PREFIXES: &[char] = &['-', '+', '*'];

impl<'a, F, T> MarkdownParser<'a, F>
where
    F: FnMut(MarkdownElement<'a>) -> Result<(), T>,
{
    /// Parse source using callback
    /// # Errors
    /// errors for unclosed blocks
    #[allow(clippy::result_unit_err, clippy::too_many_lines)]
    pub fn parse_with_options(
        on: &'a str,
        options: ParseOptions,
        container_residue: ContainerResidue<'a>,
        cb: F,
    ) -> Result<(), MarkdownParseError<T>> {
        let mut parser = MarkdownParser {
            reader: utilities::EdibleLines::new(on),
            state: State::None,
            options,
            cb,
            container_residue,
        };
        while let Some(section) = parser.reader.next() {
            parser.classify_section(section)?;
        }
        if !matches!(parser.state, State::None) {
            eprintln!("TODO state {state:?} error", state = parser.state);
        }
        Ok(())
    }

    fn classify_section(&mut self, section: &'a str) -> Result<(), MarkdownParseError<T>> {
        macro_rules! invoke {
            ($element:expr) => {
                if let Err(err) = (self.cb)($element) {
                    return Err(MarkdownParseError::FromCallback(err));
                } else {
                    self.state = State::None;
                    self.reader.moving_on();
                }
            };
        }

        let line = section.lines().next_back().unwrap_or(section);
        let line = self.container_residue.strip_prefix(line);

        match self.state {
            State::Quote { alert } => {
                let peek = self.reader.peek_line();
                if peek
                    .map(str::trim_start)
                    .map(|line| self.container_residue.strip_prefix(line))
                    .is_none_or(|line| line.is_empty() || line.starts_with('#'))
                {
                    let quote_block = QuoteBlock {
                        alert,
                        inner: RawMarkdown(section, self.container_residue),
                    };
                    invoke!(MarkdownElement::Quote(quote_block));
                }
            }
            State::List => {
                let peek = self.reader.peek_line();
                if peek
                    .map(str::trim_start)
                    .map(|line| self.container_residue.strip_prefix(line))
                    .is_none_or(|line| line.is_empty() || line.starts_with('#'))
                {
                    let list = List(RawMarkdown(section, self.container_residue));
                    invoke!(MarkdownElement::List(list));
                }
            }
            State::CodeBlock {
                language,
                delimeter,
            } => {
                let code = if let "    " = delimeter {
                    let r#continue = self
                        .reader
                        .peek_line()
                        .map(|line| self.container_residue.strip_prefix(line))
                        .is_some_and(|line| {
                            line.is_empty() || line.starts_with("    ") || line.starts_with('\t')
                        });
                    (!r#continue).then_some((true, section))
                } else if line.trim_end() == delimeter {
                    Some((false, section.rsplit_once("\n").unwrap().0))
                } else {
                    None
                };
                if let Some((indented_block, code)) = code {
                    let code_block = CodeBlock {
                        language,
                        indented_block,
                        code: code.trim_end(),
                    };
                    invoke!(MarkdownElement::CodeBlock(code_block));
                }
            }
            State::Command { name, arguments } => {
                if let Some(command_line) = utilities::strip_surrounds(line, "{%", "%}") {
                    let is_end_of_current_command = command_line
                        .trim()
                        .strip_prefix('/')
                        .is_some_and(|command| name == command);

                    if is_end_of_current_command {
                        let (inner, _) = section.rsplit_once("\n").unwrap();
                        let inner = RawMarkdown(inner, Default::default());
                        let command_block = CommandBlock {
                            name,
                            arguments,
                            inner,
                        };
                        invoke!(MarkdownElement::CommandBlock(command_block));
                    }
                }
            }
            State::MathematicsBlock => {
                if let "$$" = line.trim() {
                    let (script, _) = section.rsplit_once("\n").unwrap();
                    let mathematics_block = BlockMathematics(script);
                    invoke!(MarkdownElement::BlockMathematics(mathematics_block));
                }
            }
            State::MarkdownComment => {
                if line.trim_end().ends_with("%%") {
                    let (comment, _) = section.rsplit_once("\n").unwrap();
                    invoke!(MarkdownElement::CommentBlock(comment));
                }
            }
            State::CommandHeader => {
                if let Some(command_line) = line.strip_prefix("%}") {
                    if let Some(command_line) = command_line.strip_suffix('/') {
                        let (current_command, arguments) =
                            command_line.split_once(' ').unwrap_or((command_line, ""));
                        let command_block = CommandBlock {
                            name: current_command,
                            arguments,
                            inner: RawMarkdown("", Default::default()),
                        };
                        invoke!(MarkdownElement::CommandBlock(command_block));
                    } else {
                        let (name, arguments) =
                            command_line.split_once(' ').unwrap_or((command_line, ""));
                        self.state = State::Command { name, arguments };
                    }
                }
            }
            State::Table => {
                if self
                    .reader
                    .peek_line()
                    .is_none_or(|line| !line.trim_end().ends_with('|'))
                {
                    let table = Table(section.trim());
                    invoke!(MarkdownElement::Table(table));
                }
            }
            State::Frontmatter => {
                if let "---" = line.trim() {
                    let frontmatter = Frontmatter(&section);
                    invoke!(MarkdownElement::Frontmatter(frontmatter));
                }
            }
            State::HTMLElement { tag_name } => {
                if line.trim_end().ends_with('>') {
                    let is_comment_end = tag_name == "!--" && line.ends_with("-->");
                    let is_tag_end = line
                        .rsplit_once("</")
                        .and_then(|(_, rest)| rest.strip_suffix('>'))
                        .is_some_and(|item| item == tag_name);
                    let html_balanced = utilities::html_balanced(section, tag_name);
                    if is_comment_end || (is_tag_end && html_balanced) {
                        let element = HTMLElement(section);
                        invoke!(MarkdownElement::HTMLElement(element));
                    }
                }
            }
            State::None => {
                let list_prefixes: &[char] =
                    if self.options.allow_asterisk_and_plus_as_list_prefixes {
                        EXTENDED_LIST_PREFIXES
                    } else {
                        BASIC_LIST_PREFIXES
                    };

                let is_code_block = line.starts_with("```")
                    || (self.options.tilda_code_blocks && line.starts_with("~~~"));
                if is_code_block {
                    let delimeter_char = line.chars().next().unwrap();
                    let matching = line
                        .find(|c: char| c != delimeter_char)
                        .unwrap_or(line.len());
                    // For nesting
                    let delimeter = &line[..matching];
                    // TODO other motifiers here?
                    let language = &line[matching..].trim();
                    self.reader.moving_on();
                    self.state = State::CodeBlock {
                        delimeter,
                        language,
                    };
                } else if self.options.indented_code_blocks && line.starts_with("    ") {
                    self.state = State::CodeBlock {
                        delimeter: "    ",
                        language: "",
                    };
                } else if let "$$" = line.trim() {
                    self.state = State::MathematicsBlock;
                    self.reader.moving_on();
                } else if line.starts_with('|') {
                    self.state = State::Table;
                } else if let Some(inner) = line.strip_prefix('>') {
                    let alert = utilities::strip_surrounds(inner, "[!", "]");
                    if alert.is_some() {
                        self.reader.moving_on();
                    }
                    // TODO what to do here
                    if self
                        .reader
                        .peek_line()
                        .is_some_and(|line| line.trim_start().starts_with('>'))
                    {
                        self.state = State::Quote { alert };
                    } else {
                        let quote_block = QuoteBlock {
                            alert,
                            inner: RawMarkdown(section, Default::default()),
                        };
                        invoke!(MarkdownElement::Quote(quote_block));
                    }
                } else if let Some(rest) = line.strip_prefix('<') {
                    let trimmed = line.trim_end();
                    if rest.starts_with("!--") {
                        if trimmed.ends_with("-->") {
                            let element = HTMLElement(line);
                            invoke!(MarkdownElement::HTMLElement(element));
                        } else {
                            self.state = State::HTMLElement { tag_name: "!--" }
                        }
                    } else {
                        let tag_name = rest
                            .split_once(|chr: char| !chr.is_alphanumeric())
                            .map_or(rest, |(l, _)| l);
                        let single_line = trimmed.strip_suffix('>').is_some_and(|rest| {
                            rest.rfind("</")
                                .is_some_and(|idx| &rest[2..][idx..] == tag_name)
                        });

                        if single_line {
                            let element = HTMLElement(line);
                            invoke!(MarkdownElement::HTMLElement(element));
                        } else {
                            self.state = State::HTMLElement { tag_name };
                        }
                    }
                } else if let Some(line) = line.trim_start().strip_prefix("%%") {
                    if let Some(out) = line.trim_end().strip_suffix("%%") {
                        let comment_block = MarkdownElement::CommentBlock(out.trim());
                        invoke!(comment_block);
                    } else {
                        self.state = State::MarkdownComment;
                        self.reader.moving_on();
                    }
                } else if self.reader.is_at_start() && "---" == line.trim() {
                    self.state = State::Frontmatter;
                    self.reader.moving_on();
                } else if let Some(line) = line.strip_prefix("{%").map(str::trim_end) {
                    if let Some(command_line) = line.strip_suffix("/%}").map(str::trim) {
                        let (current_command, arguments) =
                            command_line.split_once(' ').unwrap_or((command_line, ""));
                        let command_block = CommandBlock {
                            name: current_command,
                            arguments,
                            inner: RawMarkdown("", Default::default()),
                        };
                        invoke!(MarkdownElement::CommandBlock(command_block));
                    } else if let Some(command_line) = line.strip_suffix("%}").map(str::trim) {
                        let (name, arguments) =
                            command_line.split_once(' ').unwrap_or((command_line, ""));
                        self.state = State::Command { name, arguments };
                        self.reader.moving_on();
                    } else {
                        self.state = State::CommandHeader;
                    }
                } else if line
                    .strip_prefix(list_prefixes)
                    .or_else(|| utilities::strip_number_prefix(section))
                    .is_some_and(|line| line.starts_with(char::is_whitespace))
                {
                    let peek = self.reader.peek_line();
                    if peek
                        .map(str::trim_start)
                        .map(|line| self.container_residue.strip_prefix(line))
                        .is_none_or(|line| line.is_empty() || line.starts_with('#'))
                    {
                        let list = List(RawMarkdown(section, self.container_residue));
                        invoke!(MarkdownElement::List(list));
                    } else {
                        self.state = State::List;
                    }
                } else {
                    let trimmed = line.trim();

                    let item = if section.is_empty() {
                        if !self.options.record_empty_lines {
                            self.reader.moving_on();
                            return Ok(());
                        }
                        MarkdownElement::Empty
                    } else if let "---" = trimmed {
                        MarkdownElement::HorizontalRule
                    } else if trimmed.starts_with('>') {
                        MarkdownElement::Quote(QuoteBlock {
                            alert: None,
                            inner: RawMarkdown(trimmed, Default::default()),
                        })
                    } else if trimmed.starts_with('#') {
                        let level = trimmed.chars().take_while(|c| *c == '#').count();
                        // TODO trim of trailing `#`s
                        let content = trimmed[level..].trim();
                        if trimmed[level..].starts_with(char::is_whitespace) {
                            MarkdownElement::Heading {
                                level: level.try_into().expect("deep header"),
                                content: RawText(content, self.container_residue),
                            }
                        } else {
                            // Fix for tags
                            MarkdownElement::Paragraph(RawText(trimmed, self.container_residue))
                        }
                    } else if self.options.heading_underscores
                        && self
                            .reader
                            .peek_line()
                            .is_some_and(|line| line.trim_end() == "---")
                    {
                        self.reader.skip_next();
                        // Important that it is the section
                        let content = RawText(section, self.container_residue);
                        MarkdownElement::Heading {
                            // TODO?
                            level: 1,
                            content,
                        }
                    } else {
                        let continuation = line.trim_end().ends_with('\\')
                            || self
                                .reader
                                .peek_line()
                                .map(utilities::strip_upto_three_spaces)
                                .is_some_and(|rest| rest.starts_with(char::is_alphanumeric));

                        if continuation {
                            return Ok(());
                        }

                        MarkdownElement::Paragraph(RawText(section.trim(), self.container_residue))
                    };

                    invoke!(item);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct TextDecoration(pub(crate) u8);

impl std::ops::BitXor for TextDecoration {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for TextDecoration {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl std::ops::BitAnd for TextDecoration {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for TextDecoration {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl TextDecoration {
    pub const NONE: TextDecoration = TextDecoration(0);

    // Basic decoration
    pub const BOLD: TextDecoration = TextDecoration(1 << 0);
    pub const EMPHASIS: TextDecoration = TextDecoration(1 << 1);

    // More
    pub const HIGHLIGHTED: TextDecoration = TextDecoration(1 << 2);
    pub const STRIKETHROUGH: TextDecoration = TextDecoration(1 << 3);

    // Above or below
    pub const SUPERSCRIPT: TextDecoration = TextDecoration(1 << 4);
    pub const SUBSCRIPT: TextDecoration = TextDecoration(1 << 5);

    #[must_use]
    pub fn contains(self, other: TextDecoration) -> bool {
        (self.0 & other.0) != 0
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub enum MarkdownPart<'a> {
    #[default]
    Plain,
    InlineCode,
    InlineMathematics,
    Emoji,
    Tag,
    Interpolation,
    // Links
    /// `<...>`
    RawLink,
    InternalLink {
        to: &'a str,
    },
    ExternalLink {
        to: &'a str,
    },
    // Alt text = on
    MediaLink {
        source: &'a str,
    },
    LineBreak,
    HTMLElement(HTMLElement<'a>),
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownTextElement<'a> {
    pub on: &'a str,
    pub decoration: TextDecoration,
    pub kind: MarkdownPart<'a>,
}

/// Abstraction for iterating over markdown content sections giving decoration (bold, links, etc) information
/// TODO I think this needs paragraph stripping
#[allow(clippy::struct_excessive_bools)]
pub struct PartsIterator<'a> {
    on: &'a str,
    last: usize,
    decoration: TextDecoration,
    container_residue: ContainerResidue<'a>,
}

impl<'a> PartsIterator<'a> {
    #[must_use]
    pub fn new(on: &'a str, container_residue: ContainerResidue<'a>) -> Self {
        Self {
            on: container_residue.strip_prefix(on).trim(),
            last: 0,
            decoration: TextDecoration::default(),
            container_residue,
        }
    }
}

impl<'a> Iterator for PartsIterator<'a> {
    // TODO Result
    type Item = MarkdownTextElement<'a>;

    #[allow(clippy::too_many_lines)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.last >= self.on.len() {
            None
        } else {
            let range = &self.on[self.last..];
            let first_char = range.chars().next()?;
            let after = &range[1..];
            let mut escaped = false;
            match first_char {
                '$' => {
                    let idx = after.find('$');
                    let Some(idx) = idx else {
                        todo!("error: could not find '$' in {range:?}");
                    };
                    self.last += idx + 2;
                    Some(MarkdownTextElement {
                        on: &after[..idx],
                        decoration: self.decoration,
                        kind: MarkdownPart::InlineMathematics,
                    })
                }
                '`' => {
                    let depth = range.find(|c: char| c != '`').unwrap_or(range.len() / 2);
                    let delimeter = &range[..depth];
                    let idx = after.find(delimeter);
                    let Some(idx) = idx else {
                        todo!("error: count not find '{delimeter}' in {after:?}");
                    };
                    let on = &range[delimeter.len()..(idx + 1)];
                    // https://spec.commonmark.org/0.31.2/#example-329
                    let blank = on.chars().all(|chr| chr == ' ');
                    let on = if blank {
                        on
                    } else {
                        on.strip_prefix(' ')
                            .and_then(|rest| rest.strip_suffix(' '))
                            .unwrap_or(on)
                    };
                    self.last += idx + delimeter.len() * 2;
                    Some(MarkdownTextElement {
                        on,
                        decoration: self.decoration,
                        kind: MarkdownPart::InlineCode,
                    })
                }
                '#' => {
                    let idx = after
                        .find(|chr: char| chr.is_whitespace())
                        .unwrap_or(after.len());
                    self.last += idx + 1;
                    Some(MarkdownTextElement {
                        on: &after[..idx],
                        decoration: self.decoration,
                        kind: MarkdownPart::Tag,
                    })
                }
                '\\' if utilities::starts_with_new_line_sequence(&range[1..]) => {
                    self.last += 1 + utilities::count_new_line_sequence(&range[1..]);
                    self.last += self
                        .container_residue
                        .count_skippable(&self.on[self.last..]);
                    Some(MarkdownTextElement {
                        on: "",
                        decoration: self.decoration,
                        kind: MarkdownPart::LineBreak,
                    })
                }
                '{' => {
                    let mut brace_depth: u8 = 0;
                    // TODO move
                    for (idx, matched) in after.match_indices(&['{', '}']) {
                        brace_depth = match matched {
                            "}" => brace_depth.saturating_sub(1),
                            "{" => brace_depth + 1,
                            matched => unreachable!("{matched}"),
                        };
                        if brace_depth == 0 {
                            self.last += idx + 2;
                            return Some(MarkdownTextElement {
                                on: &after[..idx],
                                decoration: self.decoration,
                                kind: MarkdownPart::Interpolation,
                            });
                        }
                    }
                    todo!("error")
                }
                ':' if after.starts_with(char::is_alphanumeric) => {
                    // TODO utilities
                    let Some(idx) = range[1..].find(':').map(|idx| idx + 1) else {
                        todo!("error")
                    };
                    self.last += idx + 1;
                    Some(MarkdownTextElement {
                        on: &range[1..idx],
                        decoration: self.decoration,
                        kind: MarkdownPart::Emoji,
                    })
                }
                '<' if after
                    .starts_with(|chr: char| chr.is_alphanumeric() || matches!(chr, '!')) =>
                {
                    let scheme_space = after.get(..32).unwrap_or(after);
                    let is_scheme = scheme_space.find(':').is_some_and(|idx| {
                        scheme_space[..idx]
                            .chars()
                            .all(|chr| chr.is_alphanumeric() || matches!(chr, '+' | '.' | '-'))
                    });
                    if is_scheme {
                        let Some(idx) = range.find('>') else {
                            todo!("no end to autolink found. return error")
                        };
                        self.last += idx + 1;
                        let inner = &range[1..idx];
                        Some(MarkdownTextElement {
                            on: inner,
                            decoration: self.decoration,
                            kind: MarkdownPart::ExternalLink { to: inner },
                        })
                    } else if range.starts_with("<!--") {
                        let Some(idx) = range.find("-->") else {
                            todo!("no end to comment found. return error")
                        };
                        let inner = &range[..(idx + 3)];
                        self.last += idx + 3;
                        Some(MarkdownTextElement {
                            on: "",
                            decoration: self.decoration,
                            kind: MarkdownPart::HTMLElement(HTMLElement(inner)),
                        })
                    } else {
                        let tag_name =
                            if let Some(idx) = after.find(|chr: char| !chr.is_alphanumeric()) {
                                &after[..idx]
                            } else {
                                todo!("no error to tag name")
                            };

                        let mut depth: u8 = 0;
                        let mut last_idx = range.len();
                        for (idx, _matched) in range.match_indices('<') {
                            if range[idx..]
                                .strip_prefix("</")
                                .is_some_and(|rest| rest.starts_with(tag_name))
                            {
                                depth = depth.saturating_sub(1);
                                if depth == 0 {
                                    last_idx = idx + 2 + tag_name.len() + 1;
                                    break;
                                }
                            } else if range[idx..][1..].starts_with(tag_name) {
                                depth += 1;
                            }
                        }
                        let inner = &range[..last_idx];
                        self.last += last_idx;
                        Some(MarkdownTextElement {
                            on: "",
                            decoration: self.decoration,
                            kind: MarkdownPart::HTMLElement(HTMLElement(inner)),
                        })
                    }
                }
                '!' if range.starts_with("![") => {
                    let till_bracket = range[2..].find(']').map(|idx| idx + 2);
                    let Some(bracket_offset) = till_bracket else {
                        todo!("error: could not find ']' in {after:?}");
                    };
                    let alt_text = &range[2..bracket_offset];
                    let next = &range[(bracket_offset + 1)..];
                    assert!(next.starts_with('('));
                    let Some(parenthesis_offset) = next.find(')') else {
                        todo!("error: count not find ')' in {next:?}")
                    };
                    let source = &range[(bracket_offset + 1)..][1..parenthesis_offset];
                    self.last += 1 + bracket_offset + parenthesis_offset + 1;
                    Some(MarkdownTextElement {
                        on: alt_text,
                        decoration: self.decoration,
                        kind: MarkdownPart::MediaLink { source },
                    })
                }
                '[' => {
                    let till_bracket = after.find(|chr| chr == ']');
                    let Some(bracket_offset) = till_bracket else {
                        todo!("error");
                    };
                    let content = &after[..bracket_offset];
                    let next = &after[(bracket_offset + 1)..];
                    if next.starts_with('(') {
                        let Some(parenthesis_offset) = next[1..].find(')').map(|idx| idx + 1)
                        else {
                            todo!("error")
                        };
                        let to = &after[(bracket_offset + 1)..][1..parenthesis_offset];
                        self.last += 2 + bracket_offset + parenthesis_offset + 1;
                        Some(MarkdownTextElement {
                            on: content,
                            decoration: self.decoration,
                            kind: MarkdownPart::ExternalLink { to },
                        })
                    } else {
                        self.last += 2 + bracket_offset;
                        Some(MarkdownTextElement {
                            on: content,
                            decoration: self.decoration,
                            kind: MarkdownPart::InternalLink { to: content },
                        })
                    }
                }
                _ => {
                    let kind = MarkdownPart::Plain;

                    for (idx, chr) in range.char_indices() {
                        if escaped {
                            escaped = false;
                            continue;
                        }
                        let mut offset = idx;
                        let current_decoration = self.decoration;
                        let left = &range[idx..];
                        let r#yield = match chr {
                            '$' | '`' | '#' | '{' | '[' => true,
                            '!' => left.starts_with("!["),
                            ':' => left.chars().nth(1).is_some_and(char::is_alphanumeric),
                            '_' | '*' => {
                                // FUTURE lots invalid here
                                if left.starts_with("___") || left.starts_with("***") {
                                    self.decoration ^= TextDecoration::BOLD;
                                    self.decoration ^= TextDecoration::EMPHASIS;
                                    offset += 3;
                                } else if left.starts_with("__") || left.starts_with("**") {
                                    self.decoration ^= TextDecoration::BOLD;
                                    offset += 2;
                                } else {
                                    self.decoration ^= TextDecoration::EMPHASIS;
                                    offset += 1;
                                }
                                true
                            }
                            '^' => {
                                self.decoration ^= TextDecoration::SUPERSCRIPT;
                                offset += 1;
                                true
                            }
                            '~' => {
                                if left.starts_with("~~") {
                                    self.decoration ^= TextDecoration::STRIKETHROUGH;
                                    offset += 2;
                                } else {
                                    self.decoration ^= TextDecoration::SUBSCRIPT;
                                    offset += 1;
                                }
                                true
                            }
                            '<' => left[1..].starts_with(|chr: char| {
                                chr.is_alphanumeric() || matches!(chr, '!')
                            }),
                            '=' if left.starts_with("==") => {
                                self.decoration ^= TextDecoration::HIGHLIGHTED;
                                offset += 2;
                                true
                            }
                            '\\' => utilities::starts_with_new_line_sequence(&left[1..]),
                            '\r' | '\n' if utilities::starts_with_new_line_sequence(&left) => {
                                let new_line_character_count =
                                    utilities::count_new_line_sequence(&left);
                                offset += new_line_character_count
                                    + self
                                        .container_residue
                                        .count_skippable(&left[new_line_character_count..]);
                                true
                            }
                            _ => false,
                        };
                        if r#yield {
                            self.last += offset;
                            return if range[..idx].is_empty() {
                                // cheeky way to jump to top
                                // TODO this can cause recursion
                                self.next()
                            } else {
                                Some(MarkdownTextElement {
                                    on: &range[..idx],
                                    decoration: current_decoration,
                                    kind,
                                })
                            };
                        }
                        escaped = chr == '\\';
                    }

                    self.last = self.on.len();

                    if !self.decoration.is_empty() {
                        eprintln!("error during parts: {:?}", self.decoration);
                    }

                    if range.trim().is_empty() {
                        None
                    } else {
                        Some(MarkdownTextElement {
                            on: range.trim_end(),
                            decoration: self.decoration,
                            kind,
                        })
                    }
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct RawMarkdown<'a>(pub &'a str, pub ContainerResidue<'a>);

#[derive(Debug, Copy, Clone)]
pub struct Table<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct CommandBlock<'a> {
    pub name: &'a str,
    pub arguments: &'a str,
    pub inner: RawMarkdown<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct Frontmatter<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct QuoteBlock<'a> {
    /// [See GitHub markdown alerts](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts).
    /// Note this allows any alerts. It does not check from a defined list
    pub alert: Option<&'a str>,
    pub inner: RawMarkdown<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct CodeBlock<'a> {
    pub indented_block: bool,
    pub language: &'a str,
    pub code: &'a str,
}

#[derive(Debug, Copy, Clone)]
pub struct HTMLElement<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct BlockMathematics<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct List<'a>(pub(crate) RawMarkdown<'a>);

#[derive(Debug, Copy, Clone)]
pub struct ListItem<'a> {
    pub content: RawMarkdown<'a>,
    /// TODO probably need more options here
    pub enumerated: bool,
    /// from `- [x]` etc
    pub checked: Option<bool>,
}

impl<'a> List<'a> {
    pub fn parse_inner(&self, mut cb: impl FnMut(crate::ListItem<'a>)) {
        let mut lines = utilities::EdibleLines::new(self.0 .0);
        while let Some(section) = lines.next() {
            let r#yield = lines
                .peek_line()
                .map(|line| self.0 .1.strip_prefix(line))
                .is_none_or(|line| {
                    line.is_empty()
                        || line.starts_with(crate::EXTENDED_LIST_PREFIXES)
                        || utilities::strip_number_prefix(line).is_some()
                });
            if r#yield {
                let trimmed = section.trim_start();
                let (content, enumerated) =
                    if let Some(rest) = trimmed.strip_prefix(crate::EXTENDED_LIST_PREFIXES) {
                        (rest, false)
                    } else if let Some(rest) = utilities::strip_number_prefix(trimmed) {
                        (rest, true)
                    } else {
                        unreachable!("{trimmed}")
                    };
                let trimmed = content.trim_start();
                let after = section.len() - trimmed.len();
                // WIP
                let prefix = &section[..after].trim_start();
                let (content, checked) = if let Some(rest) = trimmed.strip_prefix("[x]") {
                    (rest, Some(true))
                } else if let Some(rest) = trimmed.strip_prefix("[ ]") {
                    (rest, Some(false))
                } else {
                    (content, None)
                };
                let content = RawMarkdown(content, ContainerResidue(prefix));
                let list_item = crate::ListItem {
                    content,
                    enumerated,
                    checked,
                };
                lines.moving_on();
                cb(list_item)
            }
        }
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ContainerResidue<'a>(pub &'a str);

impl<'a> ContainerResidue<'a> {
    pub fn strip_prefix<'b>(&self, on: &'b str) -> &'b str {
        let mut acc = 0;
        if self.0.starts_with('-') {
            return on.trim_start();
        } else if self.0.starts_with('>') {
            if let Some(after) = on.trim_start().strip_prefix('>') {
                return after;
            } else {
                dbg!(self.0, on);
                return on.trim_start();
            }
        } else if !self.0.is_empty() {
            dbg!(self.0);
        }
        // for (idx, matched) in self.0.match_indices(&['>', '-']) {
        //     for (n, c) in on[..idx].char_indices() {
        //         if !c.is_whitespace() {
        //             if on[idx..][n..].starts_with(matched) {
        //                 acc = idx + 1;
        //             } else {
        //                 dbg!(&on[..idx], matched, &on[..idx][n..]);
        //             }
        //         } else {
        //             dbg!(&on[..idx], matched, &on[..idx][n..]);
        //             break;
        //         }
        //     }
        // }
        // dbg!(self, on, acc);
        &on[acc..]
    }

    pub fn count_skippable<'b>(&self, on: &'b str) -> usize {
        on.len() - self.strip_prefix(on).len()
    }
}
