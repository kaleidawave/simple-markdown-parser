use super::{
    utilities, CodeBlock, CommandBlock, Frontmatter, HTMLElement, List, MarkdownElement,
    MarkdownParseError, MarkdownPart, MarkdownTextElement, MathematicsBlock, QuoteBlock,
    RawMarkdown, RawText, Table, TextDecoration,
};

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

    #[allow(clippy::too_many_lines)]
    fn classify_section(&mut self, section: &'a str) -> Result<(), MarkdownParseError<T>> {
        macro_rules! invoke {
            ($element:expr) => {{
                if let Err(err) = (self.cb)($element) {
                    return Err(MarkdownParseError::FromCallback(err));
                }
                self.state = State::None;
                self.reader.moving_on();
            }};
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
                    let new_container_residue = self.container_residue.new_in_quote(section);
                    let quote_block = QuoteBlock {
                        alert,
                        inner: RawMarkdown(section, new_container_residue),
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
                    if let Some((code, _)) = section.rsplit_once('\n') {
                        Some((false, code))
                    } else {
                        // panic!("Content was {section:?}");
                        Some((false, ""))
                    }
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
                        let (inner, _) = section.rsplit_once('\n').unwrap();
                        let inner = RawMarkdown(inner, ContainerResidue::default());
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
                    let (script, _) = section.rsplit_once('\n').unwrap();
                    let mathematics_block = MathematicsBlock(script);
                    invoke!(MarkdownElement::MathematicsBlock(mathematics_block));
                }
            }
            State::MarkdownComment => {
                if line.trim_end().ends_with("%%") {
                    let (comment, _) = section.rsplit_once('\n').unwrap();
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
                            inner: RawMarkdown("", ContainerResidue::default()),
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
                    let frontmatter = Frontmatter(section);
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
                        let new_container_residue = self.container_residue.new_in_quote(section);
                        let quote_block = QuoteBlock {
                            alert,
                            inner: RawMarkdown(section, new_container_residue),
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
                            inner: RawMarkdown("", ContainerResidue::default()),
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
                                .map(|line| self.container_residue.strip_prefix(line))
                                .is_some_and(|rest| {
                                    rest.starts_with(char::is_alphanumeric)
                                        || rest.starts_with(['[', '!'])
                                });

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
                    let on = &range[delimeter.len()..=idx];
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
                    let alt = &range[2..bracket_offset];
                    let next = &range[(bracket_offset + 1)..];
                    assert!(next.starts_with('('));
                    let Some(parenthesis_offset) = next.find(')') else {
                        todo!("error: count not find ')' in {next:?}")
                    };
                    let source = &range[(bracket_offset + 1)..][1..parenthesis_offset];
                    self.last += 1 + bracket_offset + parenthesis_offset + 1;
                    Some(MarkdownTextElement {
                        on: "",
                        decoration: self.decoration,
                        kind: MarkdownPart::MediaLink { source, alt },
                    })
                }
                '[' => {
                    let mut till_bracket = None;
                    let mut depth = 1;
                    // TODO more escapes and such... :(
                    for (idx, matched) in after.match_indices(&['[', ']']) {
                        if let "]" = matched {
                            depth -= 1;
                            if depth == 0 {
                                till_bracket = Some(idx);
                                break;
                            }
                        } else {
                            depth += 1;
                        }
                    }
                    let Some(bracket_offset) = till_bracket else {
                        todo!("error");
                    };
                    let content = &after[..bracket_offset];
                    let next = &after[(bracket_offset + 1)..];
                    if let Some(next) = next.strip_prefix('(') {
                        let Some(parenthesis_offset) = next.find(')').map(|idx| idx + 1) else {
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
                            '\r' | '\n' if utilities::starts_with_new_line_sequence(left) => {
                                let new_line_character_count =
                                    utilities::count_new_line_sequence(left);
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
impl<'a> List<'a> {
    pub fn parse_inner<T>(
        &self,
        mut cb: impl FnMut(crate::ListItem<'a>) -> Result<(), T>,
    ) -> Result<(), T> {
        let RawMarkdown(item, container_residue) = self.0;
        let mut lines = utilities::EdibleLines::new(item);
        while let Some(section) = lines.next() {
            let r#yield = lines
                .peek_line()
                .map(|line| container_residue.strip_prefix(line))
                .is_none_or(|line| {
                    line.is_empty()
                        || line.starts_with(EXTENDED_LIST_PREFIXES)
                        || utilities::strip_number_prefix(line).is_some()
                });
            if r#yield {
                let trimmed = container_residue.strip_prefix(section);
                let trimmed = utilities::strip_upto_three_spaces(trimmed);
                let (content, enumerated) =
                    if let Some(rest) = trimmed.strip_prefix(EXTENDED_LIST_PREFIXES) {
                        (rest, false)
                    } else if let Some(rest) = utilities::strip_number_prefix(trimmed) {
                        (rest, true)
                    } else {
                        unreachable!("{trimmed}")
                    };
                let trimmed = content.trim_start();
                let after = section.len() - trimmed.len();
                // WIP
                let prefix = &section[..after];
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
                cb(list_item)?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Default, Debug)]
pub struct ContainerResidue<'a>(pub &'a str);

impl<'a> ContainerResidue<'a> {
    #[must_use]
    pub fn strip_prefix<'b>(&self, on: &'b str) -> &'b str {
        let Self(residue) = *self;
        // TODO `trim_start`?
        // TODO loop
        let origin = residue.trim_start();
        if origin.starts_with('>') {
            let mut expected = residue;
            let mut cur = on;
            while let (Some(next_expected), Some(next_cur)) = (
                expected.trim_start().strip_prefix('>'),
                utilities::strip_upto_three_spaces(cur).strip_prefix('>'),
            ) {
                expected = next_expected;
                cur = next_cur;
            }
            utilities::strip_upto_three_spaces(cur)
        } else if origin.strip_prefix(EXTENDED_LIST_PREFIXES).is_some()
            || utilities::strip_number_prefix(origin).is_some()
        {
            // TODO need count spaces rather than strip prefix
            let on = on
                .strip_prefix(&residue[..(residue.len() - origin.len())])
                .unwrap_or(on);
            on.strip_prefix('\t')
                .or_else(|| on.strip_prefix("  "))
                .unwrap_or(on)
        } else {
            if !residue.is_empty() {
                eprintln!("prefix was {residue}");
            }
            utilities::strip_upto_three_spaces(on)
        }
    }

    // #[must_use]
    // pub fn strip_prefix<'b>(&self, mut on: &'b str) -> &'b str {
    //     let Self(residue) = *self;
    //     let mut origin = residue.trim_start();
    //     for (_idx, _matched_indices) in origin.match_indices(&['>', '-', '*', '+']) {
    //         if origin.starts_with('>') {
    //             while let (Some(next_expected), Some(next_on)) = (
    //                 origin.trim_start().strip_prefix('>'),
    //                 utilities::strip_upto_three_spaces(on).strip_prefix('>'),
    //             ) {
    //                 origin = next_expected;
    //                 on = next_on;
    //             }
    //         } else if origin.strip_prefix(EXTENDED_LIST_PREFIXES).is_some() ||
    //             utilities::strip_number_prefix(origin).is_some()
    //         {
    //             // TODO need count spaces rather than strip prefix
    //             let on = on
    //                 .strip_prefix(&origin[..(origin.len() - origin.len())])
    //                 .unwrap_or(on);
    //             let on = on
    //                 .strip_prefix('\t')
    //                 .or_else(|| on.strip_prefix("  "))
    //                 .unwrap_or(on);
    //             return utilities::strip_upto_three_spaces(on);
    //         }
    //     }
    //     if !residue.is_empty() {
    //         eprintln!("prefix was {residue}");
    //     }
    //     utilities::strip_upto_three_spaces(on)
    // }

    #[must_use]
    pub fn count_skippable(&self, on: &str) -> usize {
        on.len() - self.strip_prefix(on).len()
    }

    #[must_use]
    pub fn new_in_quote(&self, section: &'a str) -> Self {
        let after = self
            .strip_prefix(section)
            .trim_start()
            .strip_prefix('>')
            .unwrap();
        let diff = section.len() - after.len();
        Self(&section[..diff])
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
            .map(|part| crate::RawText(part, ContainerResidue::default()))
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
