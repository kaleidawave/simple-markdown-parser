#![doc = include_str!("../README.md")]

pub mod extras;
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
    ListItem {
        level: u8,
        content: RawText<'a>,
        /// TODO probably need more options here
        enumerated: bool,
        /// from `- [x]` etc
        checked: Option<bool>,
    },
    Table(Table<'a>),
    // TODO modifiers
    CodeBlock(CodeBlock<'a>),
    BlockMathematics {
        script: &'a str,
    },
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawText<'a>(pub &'a str);

impl<'a> RawText<'a> {
    #[must_use]
    pub fn parts(&self) -> PartsIterator<'a> {
        PartsIterator::new(self.0)
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
    parse_with_options(on, ParseOptions::default(), 0, cb)
}

/// Parse source using callback
/// # Errors
/// errors for unclosed blocks
#[allow(clippy::result_unit_err, clippy::too_many_lines)]
pub fn parse_with_options<'a, T>(
    on: &'a str,
    options: ParseOptions,
    quote_depth: u8,
    mut cb: impl FnMut(MarkdownElement<'a>) -> Result<(), T>,
) -> Result<(), MarkdownParseError<T>> {
    #[allow(clippy::needless_lifetimes)]
    fn classify_line(item: &str, options: ParseOptions) -> MarkdownElement<'_> {
        let trimmed = item.trim();

        let list_prefixes: &[char] = if options.allow_asterisk_and_plus_as_list_prefixes {
            &['-', '+', '*']
        } else {
            &['-']
        };

        if trimmed.is_empty() {
            MarkdownElement::Empty
        } else if let "---" = trimmed {
            MarkdownElement::HorizontalRule
        } else if trimmed.starts_with('>') {
            MarkdownElement::Quote(QuoteBlock {
                alert: None,
                inner: trimmed,
            })
        } else if trimmed.starts_with('#') {
            let level = trimmed.chars().take_while(|c| *c == '#').count();
            // TODO trim of trailing `#`s
            let content = trimmed[level..].trim();
            if trimmed[level..].starts_with(char::is_whitespace) {
                MarkdownElement::Heading {
                    level: level.try_into().expect("deep header"),
                    content: RawText(content),
                }
            } else {
                // Fix for tags
                MarkdownElement::Paragraph(RawText(trimmed))
            }
        } else if let Some(trimmed) = trimmed.trim_start().strip_prefix(list_prefixes) {
            // TODO one or the other
            let level = item.chars().take_while(|c| *c == '\t' || *c == ' ').count();
            let (checked, trimmed) = if let Some(left) = trimmed.trim_start().strip_prefix("[x]") {
                (Some(true), left)
            } else if let Some(left) = trimmed.trim_start().strip_prefix("[ ]") {
                (Some(false), left)
            } else {
                (None, trimmed.trim())
            };

            MarkdownElement::ListItem {
                level: level.try_into().expect("deep list item"),
                content: RawText(trimmed),
                enumerated: false,
                checked,
            }
        } else if let Some(trimmed) = utilities::strip_number_prefix(trimmed) {
            let level = item.chars().take_while(|c| *c == '\t' || *c == ' ').count();
            MarkdownElement::ListItem {
                // TODO take number
                level: level.try_into().expect("deep list item"),
                content: RawText(trimmed.trim()),
                enumerated: true,
                checked: None,
            }
        } else {
            MarkdownElement::Paragraph(RawText(trimmed))
        }
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
        HTMLElement {
            tag_name: &'a str,
            // FUTURE a bit of a hack
            depth: usize,
        },
    }

    // TODO explain difference between upto last_line and end...
    let mut upto = 0;
    let mut last_line = 0;

    let mut state = State::None;

    let char_indices = on.char_indices().chain([(on.len(), '\n')]);

    for (idx, chr) in char_indices {
        if let '\n' = chr {
            let mut line = &on[last_line..idx];
            if let State::None = state {
                // FUTURE needs improving
                let continuation = line.starts_with(|chr: char| chr.is_alphabetic())
                    && (line.trim_end().ends_with('\\')
                        || on.get((idx + 1)..).is_some_and(|rest| {
                            utilities::strip_upto_three_spaces(rest)
                                .starts_with(|chr: char| chr.is_alphanumeric())
                        }));

                if continuation {
                    continue;
                }
                if on[last_line..].is_empty() {
                    break;
                }
            }

            for _ in 0..quote_depth {
                // FUTURE unwrap?
                line = line.strip_prefix('>').unwrap_or(line);
            }

            // This is done here because more might need to present here
            if let State::CodeBlock {
                language: _,
                delimeter: "    ",
            } = state
            {
                let r#continue =
                    line.is_empty() || line.starts_with("    ") || line.starts_with('\t');
                if r#continue {
                    last_line = idx + '\n'.len_utf8();
                    continue;
                }

                // Important that take is done to reset option
                let code_block = MarkdownElement::CodeBlock(CodeBlock {
                    language: "",
                    code: &on[upto..last_line],
                });
                cb(code_block).map_err(MarkdownParseError::FromCallback)?;
                upto = last_line;
                state = State::None;
            }

            match state {
                State::Quote { alert } => {
                    if line.starts_with("> ") {
                        last_line = idx + '\n'.len_utf8();
                        continue;
                    }
                    let inner = &on[upto..idx].trim_end();
                    let quote_block = QuoteBlock { alert, inner };
                    cb(MarkdownElement::Quote(quote_block))
                        .map_err(MarkdownParseError::FromCallback)?;
                    upto = last_line;
                    state = State::None;
                }
                State::CodeBlock {
                    language,
                    delimeter,
                } => {
                    if line.starts_with(delimeter) {
                        // Important that take is done to reset option
                        let code_block = MarkdownElement::CodeBlock(CodeBlock {
                            language,
                            code: &on[upto..last_line],
                        });
                        cb(code_block).map_err(MarkdownParseError::FromCallback)?;
                        upto = last_line;
                        state = State::None;
                    }
                }
                State::Command { name, arguments } => {
                    if let Some(command_line) = utilities::strip_surrounds(line, "{%", "%}") {
                        let is_command = command_line
                            .trim()
                            .strip_prefix('/')
                            .is_some_and(|command| name == command);

                        if is_command {
                            let command_block = MarkdownElement::CommandBlock(CommandBlock {
                                name,
                                arguments,
                                inner: RawMarkdown(&on[upto..last_line]),
                            });
                            cb(command_block).map_err(MarkdownParseError::FromCallback)?;
                            upto = last_line;
                            state = State::None;
                        }
                    }
                }
                State::MathematicsBlock => {
                    if let "$$" = line.trim() {
                        let mathematics_block = MarkdownElement::BlockMathematics {
                            script: on[upto..last_line].trim(),
                        };
                        cb(mathematics_block).map_err(MarkdownParseError::FromCallback)?;
                        upto = last_line;
                        state = State::None;
                    }
                }
                State::MarkdownComment => {
                    if line.trim_end().ends_with("%%") {
                        let comment_block =
                            MarkdownElement::CommentBlock(on[upto..last_line].trim());
                        cb(comment_block).map_err(MarkdownParseError::FromCallback)?;
                        upto = last_line;
                        state = State::None;
                    }
                }
                State::CommandHeader => {
                    if let Some(command_line) = line.strip_prefix("%}") {
                        if let Some(command_line) = command_line.strip_suffix('/') {
                            let (current_command, arguments) =
                                command_line.split_once(' ').unwrap_or((command_line, ""));
                            let command_block = MarkdownElement::CommandBlock(CommandBlock {
                                name: current_command,
                                arguments,
                                inner: RawMarkdown(""),
                            });
                            cb(command_block).map_err(MarkdownParseError::FromCallback)?;
                            upto = last_line;
                            state = State::None;
                        } else {
                            let (name, arguments) =
                                command_line.split_once(' ').unwrap_or((command_line, ""));
                            state = State::Command { name, arguments };
                        }
                    }
                }
                State::Table => {
                    if !line.trim_end().ends_with('|') {
                        let table = MarkdownElement::Table(Table(on[upto..last_line].trim()));
                        cb(table).map_err(MarkdownParseError::FromCallback)?;
                        state = State::None;
                    }
                }
                State::Frontmatter => {
                    let is_horizontal_rule = "---" == line.trim();
                    if is_horizontal_rule {
                        let frontmatter =
                            MarkdownElement::Frontmatter(Frontmatter(&on[upto..last_line]));
                        cb(frontmatter).map_err(MarkdownParseError::FromCallback)?;
                        state = State::None;
                        upto = idx + 1;
                    }
                }
                State::HTMLElement {
                    tag_name,
                    ref mut depth,
                } => {
                    let line = line.trim_end();
                    if let (0, Some(line)) = (*depth, line.strip_suffix('>')) {
                        let element = MarkdownElement::HTMLElement(HTMLElement(&on[upto..idx]));
                        let is_comment = tag_name == "!--" && line.ends_with("-->");
                        let r#yield = is_comment
                            || line
                                .rfind("</")
                                .is_some_and(|close| &line[2..][close..] == tag_name);
                        if r#yield {
                            cb(element).map_err(MarkdownParseError::FromCallback)?;
                            state = State::None;
                            upto = idx + 1;
                            last_line = idx + '\n'.len_utf8();
                            continue;
                        }
                    }

                    // FUTURE needs improving
                    for (idx, _) in line.match_indices("<") {
                        let rest = &line[idx..];
                        let (check, new) = if rest.starts_with("</") {
                            (&rest[2..], depth.saturating_sub(1))
                        } else {
                            (&rest[1..], *depth + 1)
                        };
                        let out = check
                            .split_once(|chr: char| !chr.is_alphanumeric())
                            .map_or(rest, |(l, _)| l);
                        if out == tag_name {
                            *depth = new;
                        }
                    }
                }
                State::None => {
                    if line.starts_with("```")
                        || (options.tilda_code_blocks && line.starts_with("~~~"))
                    {
                        let delimeter_char = line.chars().next().unwrap();
                        let matching = line
                            .find(|c: char| c != delimeter_char)
                            .unwrap_or(line.len());
                        // For nesting
                        let delimeter = &line[..matching];
                        // TODO other motifiers here?
                        let language = &line[matching..].trim();
                        state = State::CodeBlock {
                            delimeter,
                            language,
                        };
                        upto = idx + '\n'.len_utf8();
                    } else if options.indented_code_blocks && line.starts_with("    ") {
                        state = State::CodeBlock {
                            delimeter: "    ",
                            language: "",
                        };
                        upto = last_line;
                    } else if let "$$" = line.trim() {
                        state = State::MathematicsBlock;
                        upto = idx + '\n'.len_utf8();
                    } else if line.starts_with('|') {
                        state = State::Table;
                        upto = last_line;
                    } else if let Some(inner) = line.strip_prefix('>') {
                        let alert = utilities::strip_surrounds(inner, "[!", "]");
                        if alert.is_some() {
                            upto = idx + 1;
                        }
                        state = State::Quote { alert };
                    } else if let Some(rest) = line.strip_prefix('<') {
                        let trimmed = line.trim_end();
                        if rest.starts_with("!--") {
                            if trimmed.ends_with("-->") {
                                let element =
                                    MarkdownElement::HTMLElement(HTMLElement(&on[upto..idx]));
                                cb(element).map_err(MarkdownParseError::FromCallback)?;
                                state = State::None;
                                upto = idx + 1;
                            } else {
                                state = State::HTMLElement {
                                    tag_name: "!--",
                                    depth: 0,
                                }
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
                                let element =
                                    MarkdownElement::HTMLElement(HTMLElement(&on[upto..idx]));
                                cb(element).map_err(MarkdownParseError::FromCallback)?;
                                state = State::None;
                                upto = idx + 1;
                            } else {
                                state = State::HTMLElement { tag_name, depth: 0 };
                            }
                        }
                    } else if let Some(line) = line.trim_start().strip_prefix("%%") {
                        if let Some(out) = line.trim_end().strip_suffix("%%") {
                            let comment_block = MarkdownElement::CommentBlock(out.trim());
                            cb(comment_block).map_err(MarkdownParseError::FromCallback)?;
                        } else {
                            state = State::MarkdownComment;
                            upto = idx + '\n'.len_utf8();
                        }
                    } else if upto == 0 && "---" == line.trim() {
                        state = State::Frontmatter;
                        upto = idx + '\n'.len_utf8();
                    } else if let Some(line) = line.strip_prefix("{%").map(str::trim_end) {
                        if let Some(command_line) = line.strip_suffix("/%}").map(str::trim) {
                            let (current_command, arguments) =
                                command_line.split_once(' ').unwrap_or((command_line, ""));
                            let command_block = MarkdownElement::CommandBlock(CommandBlock {
                                name: current_command,
                                arguments,
                                inner: RawMarkdown(""),
                            });
                            cb(command_block).map_err(MarkdownParseError::FromCallback)?;
                            upto = idx;
                        } else if let Some(command_line) = line.strip_suffix("%}").map(str::trim) {
                            upto = idx;
                            let (name, arguments) =
                                command_line.split_once(' ').unwrap_or((command_line, ""));
                            state = State::Command { name, arguments };
                        } else {
                            state = State::CommandHeader;
                        }
                    } else {
                        if options.heading_underscores
                            && !line.trim().is_empty()
                            && utilities::strip_upto_one_new_line(&on[idx..]).starts_with("---")
                        {
                            let header = MarkdownElement::Heading {
                                // FUTURE I think
                                level: 1,
                                content: RawText(line),
                            };
                            // FUTURE I think this can break
                            upto = idx + '\n'.len_utf8() + "---".len();
                            last_line = upto;
                            cb(header).map_err(MarkdownParseError::FromCallback)?;
                            continue;
                        }

                        let item = classify_line(line, options);
                        let skip =
                            !options.record_empty_lines && matches!(item, MarkdownElement::Empty);
                        if !skip {
                            cb(item).map_err(MarkdownParseError::FromCallback)?;
                        }
                        upto = idx + '\n'.len_utf8();
                    }
                }
            }

            last_line = idx + '\n'.len_utf8();
        }
    }

    if !matches!(state, State::None) {
        eprintln!("TODO state {state:?} error");
    }

    Ok(())
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
    HTMLElement,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownTextElement<'a> {
    pub on: &'a str,
    pub decoration: TextDecoration,
    pub kind: MarkdownPart<'a>,
}

/// Abstraction for iterating over markdown content sections giving decoration (bold, links, etc) information
#[allow(clippy::struct_excessive_bools)]
pub struct PartsIterator<'a> {
    on: &'a str,
    // Internal state
    last: usize,
    // Decoration state
    decoration: TextDecoration,
}

impl<'a> PartsIterator<'a> {
    #[must_use]
    pub fn new(on: &'a str) -> Self {
        Self {
            on: on.trim_start(),
            last: 0,
            decoration: TextDecoration::default(),
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
                '{' => {
                    let mut brace_depth: u8 = 0;
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
                        self.last += idx + 3;
                        let inner = &range[..(idx + 3)];
                        Some(MarkdownTextElement {
                            on: inner,
                            decoration: self.decoration,
                            kind: MarkdownPart::HTMLElement,
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
                            on: inner,
                            decoration: self.decoration,
                            kind: MarkdownPart::HTMLElement,
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
                        } else {
                            let mut offset = idx;
                            let current_decoration = self.decoration;
                            let r#yield = match chr {
                                '$' | '`' | '#' | '{' | '[' => true,
                                '!' => range[idx..].starts_with("!["),
                                ':' => range[idx..]
                                    .chars()
                                    .nth(1)
                                    .is_some_and(char::is_alphanumeric),
                                '_' | '*' => {
                                    // FUTURE lots invalid here
                                    let left = &range[idx..];
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
                                    if range[idx..].starts_with("~~") {
                                        self.decoration ^= TextDecoration::STRIKETHROUGH;
                                        offset += 2;
                                    } else {
                                        self.decoration ^= TextDecoration::SUBSCRIPT;
                                        offset += 1;
                                    }
                                    true
                                }
                                '<' => range[idx..][1..].starts_with(|chr: char| {
                                    chr.is_alphanumeric() || matches!(chr, '!')
                                }),
                                '=' if range[idx..].starts_with("==") => {
                                    self.decoration ^= TextDecoration::HIGHLIGHTED;
                                    offset += 2;
                                    true
                                }
                                _ => false,
                            };
                            if r#yield {
                                self.last += offset;
                                return if range[..idx].is_empty() {
                                    // cheeky way to jump to top
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
                    }

                    self.last = self.on.len();

                    if !self.decoration.is_empty() {
                        eprintln!("error {:?}", self.decoration);
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawMarkdown<'a>(pub &'a str);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Table<'a>(pub &'a str);

impl<'a> Table<'a> {
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
        inner.split('|').map(crate::RawText)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CommandBlock<'a> {
    pub name: &'a str,
    pub arguments: &'a str,
    pub inner: RawMarkdown<'a>,
}

impl<'a> CommandBlock<'a> {
    #[must_use]
    #[allow(clippy::collapsible_else_if)]
    pub fn parse_arguments(&self) -> Vec<(&'a str, &'a str)> {
        crate::utilities::parse_arguments(self.arguments)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Frontmatter<'a>(pub &'a str);

impl Frontmatter<'_> {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct QuoteBlock<'a> {
    /// [See GitHub markdown alerts](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts).
    /// Note this allows any alerts. It does not check from a defined list
    pub alert: Option<&'a str>,
    pub inner: &'a str,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CodeBlock<'a> {
    pub language: &'a str,
    pub code: &'a str,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HTMLElement<'a>(pub &'a str);
