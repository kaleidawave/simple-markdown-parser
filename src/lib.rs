#![doc = include_str!("../README.md")]

pub mod extras;
pub mod utilities;

/// Markdown block element
#[derive(Debug, Clone)]
pub enum MarkdownElement<'a> {
    Heading {
        level: u8,
        text: RawText<'a>,
    },
    Quote(QuoteBlock<'a>),
    Paragraph(RawText<'a>),
    ListItem {
        level: u8,
        text: RawText<'a>,
        /// TODO probably need more options here
        enumerated: bool,
        /// from `- [x]` etc
        checked: Option<bool>
    },
    Table(Table<'a>),
    // TODO modifiers
    CodeBlock {
        language: &'a str,
        code: &'a str,
    },
    LaTeXBlock {
        script: &'a str,
    },
    CommandBlock(CommandBlock<'a>),
    /// Inside `%%` (from Obsidan)
    CommentBlock(&'a str),
    /// Includes HTML comments
    // TODO how much to do here
    #[cfg(feature = "html")]
    HTMLElement {
        element: lightml::Element<'a>,
        source: &'a str,
    },
    // TODO at start?
    Frontmatter(&'a str),
    HorizontalRule,
    // Media {
    //     alt: &'a str,
    //     link: Option<&'a str>,
    //     source: &'a str,
    // },
    Footnote,
    Empty,
}

impl<'a> MarkdownElement<'a> {
    #[must_use]
    pub fn as_markdown(&self) -> String {
        match self {
            Self::Heading { level, text } => {
                let mut s = "#".repeat(*level as usize);
                s.push_str(text.0);
                s.push(' ');
                s
            }
            Self::ListItem {
                level,
                text,
                enumerated: _,
                checked
            } => {
                // TODO enumerated
                let mut s = "\t".repeat(*level as usize);
                s.push_str("- ");
                if let Some(checked) = checked {
                    s.push_str(if *checked { "[x]" }  else { "[ ]" });
                }
                s.push_str(text.0);
                s
            }
            Self::CodeBlock { language, code } => {
                format!("```{language}\n{code}```")
                // let mut s = "```".to_owned();
                // s.push_str(language);
                // s.push_str("\n");
                // s.push_str("```");
                // s
            }
            Self::Paragraph(text) => text.0.to_owned(),
            Self::Quote(text) => text.inner.to_owned(),
            Self::Frontmatter(source) => {
                format!("---\n{source}---")
            }
            #[cfg(feature = "html")]
            Self::HTMLElement { element: _, source } => source.to_string(),
            Self::Empty => String::new(),
            item => format!("TODO {item:?}"),
        }
    }

    /// Paragraph text like elements
    #[must_use]
    pub fn inner_paragraph_raw(&self) -> Option<&str> {
        if let MarkdownElement::Paragraph(text) = self {
            Some(text.0)
        } else if let MarkdownElement::Quote(text) = self {
            // TODO these can be sometimes made up of elements
            Some(text.inner)
        } else {
            None
        }
    }

    #[must_use]
    pub fn parts_like(&self) -> Option<RawText<'a>> {
        if let MarkdownElement::Heading { text, .. }
        | MarkdownElement::Paragraph(text)
        | MarkdownElement::ListItem { text, .. } = self
        {
            Some(*text)
        } else {
            // else if let MarkdownElement::Quote(text) = self {
            //     // TODO these can be sometimes made up of elements
            //     Some(RawText(text.0))
            // }
            None
        }
    }

    #[allow(clippy::match_same_arms)]
    #[must_use]
    pub fn debug_without_text(&self) -> String {
        match self {
            MarkdownElement::Heading { level, text: _ } => {
                format!("Heading {{ level: {level} }}")
            }
            MarkdownElement::Quote(_) => "Quote".to_owned(),
            MarkdownElement::Paragraph(_) => "Paragraph".to_owned(),
            MarkdownElement::ListItem {
                level,
                text: _,
                enumerated,
                checked
            } => {
                format!("ListItem {{ level: {level}, enumerated: {enumerated:?}, checked: {checked:?} }}")
            }
            MarkdownElement::Table(_table) => "Table".to_owned(),
            MarkdownElement::CodeBlock { language, code: _ } => format!("CodeBlock ({language})"),
            MarkdownElement::LaTeXBlock { script: _ } => "LaTeXBlock {{ .. }}".to_owned(),
            MarkdownElement::CommandBlock(_) => "CommandBlock".to_owned(),
            MarkdownElement::CommentBlock(_) => "CommentBlock".to_owned(),
            MarkdownElement::Frontmatter(_) => "Frontmatter".to_owned(),
            MarkdownElement::HorizontalRule => "HorizontalRule".to_owned(),
            MarkdownElement::Footnote => "Footnote".to_owned(),
            MarkdownElement::Empty => "Empty".to_owned(),
            #[cfg(feature = "html")]
            Self::HTMLElement { .. } => "HTMLElement".to_owned(),
        }
    }
}

/// (unsplit) Text inside markdown item
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawText<'a>(pub &'a str);

impl<'a> RawText<'a> {
    #[must_use]
    pub fn parts(&self) -> PartsIterator<'a> {
        PartsIterator::new(self.0, false)
    }

    #[must_use]
    pub fn parts_whitespace(&self) -> PartsIterator<'a> {
        PartsIterator::new(self.0, true)
    }

    #[must_use]
    pub fn no_decoration(&self) -> String {
        let mut s = String::new();
        for part in PartsIterator::new(self.0, false) {
            s.push_str(part.no_decoration());
        }
        s
    }
}

/// Some are prefixes, some are wrapped
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MarkdownTextElement<'a> {
    Plain(&'a str),
    /// `*hi*` or `_hi_`
    Italic(&'a str),
    /// `**hi**` or `__hi__`
    Bold(&'a str),
    /// **_hi_**
    BoldAndItalic(&'a str),
    /// `` `code` ``
    Code(&'a str),
    /// `~~gone~~`
    StrikeThrough(&'a str),
    /// `:emoji:`
    Emoji(&'a str),
    /// `$\sin$`
    Latex(&'a str),
    /// `{something}` TODO WIP
    Expression(&'a str),
    /// `==hightlighted==`
    Highlight(&'a str),
    /// `^superscript^`
    Superscript(&'a str),
    /// `~subscript~` (unfortuantly not _)
    Subscript(&'a str),
    /// `#item`
    Tag(&'a str),
    /// `[on](to)`
    Link {
        /// TODO not great but..
        on: RawText<'a>,
        to: &'a str,
    },
    /// `![alt](source)`
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
                eprintln!("TODO no decoration link");
                ""
            }
        }
    }
}

#[derive(Default, Copy, Clone)]
pub struct ParseOptions {
    /// For formatting preservation
    pub include_new_lines: bool,
    /// Avoid
    pub allow_asterisk_and_plus_as_list_prefixes: bool,
    /// Avoid
    pub heading_underscores: bool,
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
    fn classify_line<'a>(item: &'a str, options: ParseOptions) -> MarkdownElement<'a> {
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
            if trimmed[level..].starts_with(char::is_whitespace) {
                MarkdownElement::Heading {
                    level: level.try_into().expect("deep header"),
                    text: RawText(item[level..].trim()),
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
                text: RawText(trimmed),
                enumerated: false,
                checked
            }
        } else if let Some(trimmed) = strip_number_prefix(trimmed) {
            let level = item.chars().take_while(|c| *c == '\t' || *c == ' ').count();
            MarkdownElement::ListItem {
                // TODO take number
                level: level.try_into().expect("deep list item"),
                text: RawText(trimmed.trim()),
                enumerated: true,
                checked: None
            }
        } else {
            MarkdownElement::Paragraph(RawText(trimmed))
        }
    }

    // TODO explain difference between upto last_line and end...
    let mut upto = 0;
    let mut last_line = 0;

    // Some => in_code
    // first argument holds the end
    let mut current_code_language: Option<(&str, &str)> = None;

    let mut current_command_and_arguments: Option<(&str, &str)> = None;

    let mut in_frontmatter = false;
    let mut in_table = false;
    let mut in_latex_block = false;
    let mut in_markdown_comment = false;
    let mut in_command_header = false;
    // Some => in_quote
    let mut quote_and_alert: Option<&str> = None;

    let mut char_indices = on.char_indices().chain([(on.len(), '\n')]);

    while let Some((idx, chr)) = char_indices.next() {
        if chr == '\n' {
            let mut line = &on[last_line..idx];
            for _ in 0..quote_depth {
                // TODO
                line = line.strip_prefix('>').unwrap_or(line);
            }
            let end = idx;
            
            if quote_and_alert.is_some() {
                if line.starts_with("> ") {
                    last_line = idx + '\n'.len_utf8();
                    continue;
                }
                let raw = &on[upto..end];
                let quote_block = QuoteBlock {
                    alert: quote_and_alert.take(),
                    inner: raw,
                };
                cb(MarkdownElement::Quote(quote_block))
                    .map_err(MarkdownParseError::FromCallback)?;
                upto = last_line;
            }

            if let Some((to_match, _)) = current_code_language {
                if line.trim().strip_prefix("```").is_some_and(|rest| rest == to_match) {
                    // Important that take is done to reset option
                    let current_code_language = current_code_language.take().unwrap().1;
                    cb(MarkdownElement::CodeBlock {
                        language: current_code_language,
                        code: &on[upto..last_line],
                    })
                    .map_err(MarkdownParseError::FromCallback)?;
                    upto = last_line;
                }
            } else if let Some((current_command, arguments)) = current_command_and_arguments {
                if let Some(command_line) = strip_surrounds(line, "{%", "%}") {
                    let is_command = command_line
                        .trim()
                        .strip_prefix('/')
                        .is_some_and(|command| current_command == command);

                    if is_command {
                        cb(MarkdownElement::CommandBlock(CommandBlock {
                            name: current_command,
                            arguments,
                            inner: RawMarkdown(&on[upto..last_line]),
                        }))
                        .map_err(MarkdownParseError::FromCallback)?;
                        current_command_and_arguments = None;
                        upto = last_line;
                    }
                }
            } else if in_latex_block {
                if let "$$" = line.trim() {
                    cb(MarkdownElement::LaTeXBlock {
                        script: on[upto..last_line].trim(),
                    })
                    .map_err(MarkdownParseError::FromCallback)?;
                    in_latex_block = false;
                    upto = last_line;
                }
            } else if in_markdown_comment {
                if line.trim_end().ends_with("%%") {
                    cb(MarkdownElement::CommentBlock(on[upto..last_line].trim()))
                        .map_err(MarkdownParseError::FromCallback)?;
                    in_markdown_comment = false;
                    upto = last_line;
                }
            } else if in_command_header {
                if let Some(command_line) = line.strip_prefix("%}") {
                    if let Some(command_line) = command_line.strip_suffix('/') {
                        let (current_command, arguments) =
                            command_line.split_once(' ').unwrap_or((command_line, ""));
                        cb(MarkdownElement::CommandBlock(CommandBlock {
                            name: current_command,
                            arguments,
                            inner: RawMarkdown(""),
                        }))
                        .map_err(MarkdownParseError::FromCallback)?;
                        upto = last_line;
                    } else {
                        current_command_and_arguments =
                            Some(command_line.split_once(' ').unwrap_or((command_line, "")));
                    }
                }
            } else if in_table {
                if !line.trim_end().ends_with('|') {
                    cb(MarkdownElement::Table(Table(on[upto..last_line].trim())))
                        .map_err(MarkdownParseError::FromCallback)?;
                    in_table = false;
                }
            } else if in_frontmatter {
                let is_horizontal_rule = "---" == line.trim();
                if is_horizontal_rule {
                    cb(MarkdownElement::Frontmatter(&on[upto..last_line]))
                        .map_err(MarkdownParseError::FromCallback)?;
                    in_frontmatter = false;
                    upto = end;
                }
            } else if let Some(rest) = line.trim().strip_prefix("```") {
                let matching = rest.chars().filter(|c| *c == '`').count();
                // For nesting
                let backticks = &rest[..matching];
                let trailing = &rest[matching..];
                // TODO other motifiers here?
                current_code_language = Some((backticks, trailing));
                upto = idx + '\n'.len_utf8();
            } else if let "$$" = line.trim() {
                in_latex_block = true;
                upto = idx + '\n'.len_utf8();
            } else if line.starts_with('|') {
                in_table = true;
                upto = last_line;
            } else if let Some(inner) = line.strip_prefix('>') {
                let command = strip_surrounds(inner, "[!", "]");
                if command.is_some() {
                    upto = end;
                }
                quote_and_alert = Some(command.unwrap_or_default());
            } else if let Some(line) = line.trim_start().strip_prefix("%%") {
                if let Some(out) = line.trim_end().strip_suffix("%%") {
                    let element = MarkdownElement::CommentBlock(out.trim());
                    cb(element).map_err(MarkdownParseError::FromCallback)?;
                } else {
                    in_markdown_comment = true;
                    upto = idx + '\n'.len_utf8();
                }
            } else if upto == 0 && "---" == line.trim() {
                in_frontmatter = true;
                upto = idx + '\n'.len_utf8();
            } else if let Some(line) = line.strip_prefix("{%") {
                if let Some(command_line) = line.strip_suffix("/%}").map(str::trim) {
                    let (current_command, arguments) =
                        command_line.split_once(' ').unwrap_or((command_line, ""));
                    let element = MarkdownElement::CommandBlock(CommandBlock {
                        name: current_command,
                        arguments,
                        inner: RawMarkdown(""),
                    });
                    cb(element).map_err(MarkdownParseError::FromCallback)?;
                    upto = last_line;
                    continue;
                } else if let Some(command_line) = line.strip_prefix("%}") {
                    current_command_and_arguments =
                        Some(command_line.split_once(' ').unwrap_or((command_line, "")));
                } else {
                    in_command_header = true;
                }
            } else {
                // TODO maybe a little more
                #[cfg(feature = "html")]
                if line.starts_with("<") {
                    use lightml::Element;
                    let current = &on[upto..];

                    let result = Element::from_string(current);
                    let (element, consumed) = match result {
                        Ok(result) => result,
                        Err(err) => {
                            panic!("{err:?}");
                        }
                    };
                    let source = &current[..consumed as usize];
                    // TODO eww
                    {
                        (0..source.chars().count()).for_each(|_| {
                            char_indices.next();
                        });
                    }
                    cb(MarkdownElement::HTMLElement { element, source })
                        .map_err(MarkdownParseError::FromCallback)?;
                    upto += consumed as usize;
                    continue;
                }

                // todo if options.underscore_headings && source[idx..].starts_with("---") {
                //     to header
                // }

                let result = classify_line(line, options);
                let to_add = !matches!(
                    (options.include_new_lines, &result),
                    (false, MarkdownElement::Empty)
                );
                if to_add {
                    cb(result).map_err(MarkdownParseError::FromCallback)?;
                }
                upto = idx + '\n'.len_utf8();
            }

            last_line = idx + '\n'.len_utf8();
        }
    }

    if current_code_language.is_some() {
        eprintln!("TODO error current_code_language={current_code_language:?}");
        // todo!("error here");
    } else if in_latex_block {
        eprintln!("TODO unclosed latex block");
    }

    // let line = on[upto..].trim_start();
    // if in_table {
    //     cb(MarkdownElement::Table(Table(line))).map_err(MarkdownParseError::FromCallback)?;
    // } else if let Some(content) = line.strip_prefix("> ") {
    //     let command_block = CommandBlock {
    //         name: "quote",
    //         arguments: quote_and_alert.unwrap_or_default(),
    //         inner: RawMarkdown(content),
    //     };
    //     cb(MarkdownElement::CommandBlock(command_block))
    //         .map_err(MarkdownParseError::FromCallback)?;
    // } else if let (Some(current_code_language), "```") = (current_code_language, line.trim()) {

    // } else {
    //     let result = classify_line(line, options);
    //     let to_add = !matches!(
    //         (options.include_new_lines, &result),
    //         (false, MarkdownElement::Empty)
    //     );
    //     if to_add {
    //         cb(result).map_err(MarkdownParseError::FromCallback)?;
    //     }
    // }

    Ok(())
}

/// Work in progress abstraction for iterating over markdown text sections giving decoration (bold, links, etc) information
/// TODO WIP
#[allow(clippy::struct_excessive_bools)]
pub struct PartsIterator<'a> {
    on: &'a str,
    // Ignores empty strings, trims text blocks
    preserve_whitespace: bool,
    // Internal state
    last: usize,
    in_tag: bool,
    pub in_bold: bool,
    pub in_italic: bool,
    in_code: bool,
    in_latex: bool,
    in_emoji: bool,
    in_expression: bool,
    in_chevron_link: bool,
    in_link: bool,
    in_internal_link: bool,
    in_media: bool,
}

impl<'a> PartsIterator<'a> {
    #[must_use]
    pub fn new(on: &'a str, preserve_whitespace: bool) -> Self {
        Self {
            on,
            preserve_whitespace,
            last: 0,
            in_tag: false,
            in_bold: false,
            in_italic: false,
            in_emoji: false,
            in_code: false,
            in_latex: false,
            in_link: false,
            in_chevron_link: false,
            in_internal_link: false,
            in_media: false,
            in_expression: false,
        }
    }
}

impl<'a> Iterator for PartsIterator<'a> {
    type Item = MarkdownTextElement<'a>;

    #[allow(clippy::too_many_lines)]
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
                                    self.in_media = false;
                                    return Some(MarkdownTextElement::Link {
                                        on: RawText(""),
                                        to: &range[..link_text_end],
                                    });
                                }
                                panic!("media parsing broken {chr}");
                            }
                        } else if let ')' | ']' = chr {
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
                        if self.in_internal_link && range[idx..].starts_with("]]") {
                            let element = if self.in_link {
                                self.in_link = false;
                                MarkdownTextElement::Link {
                                    on: RawText(""),
                                    to: &range[1..idx],
                                }
                            } else {
                                self.in_media = false;
                                MarkdownTextElement::Media {
                                    alt: "",
                                    source: &range[1..idx],
                                }
                            };

                            self.last += idx + 2;
                            return Some(element);
                        } else if let Some(reduced_depth) = bracket_depth.checked_sub(1) {
                            bracket_depth = reduced_depth;
                        } else {
                            link_text_end = Some(idx);
                        }
                    } else if let '[' = chr {
                        if idx == 0 {
                            self.in_internal_link = true;
                            // Reset
                            // range = &&self.on[idx..];
                        } else {
                            bracket_depth += 1;
                        }
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
                // TODO escaped stuff etc
                if self.in_chevron_link {
                    if let '>' = chr {
                        self.last += idx + 1;
                        self.in_chevron_link = false;
                        let inner = &range[..idx];
                        return Some(MarkdownTextElement::Link {
                            // presentation as same as link
                            on: RawText(inner),
                            to: inner,
                        });
                    }
                    continue;
                }

                if self.in_tag && chr.is_whitespace() {
                    self.last += idx;
                    self.in_tag = false;
                    return Some(MarkdownTextElement::Tag(&range[..idx]));
                }

                macro_rules! yield_current {
                    () => {{
                        let item = &range[..idx];
                        let item = if self.preserve_whitespace {
                            item
                        } else {
                            item.trim()
                        };
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
                    '<' if range[(idx + 1)..]
                        .chars()
                        .next()
                        .is_some_and(char::is_alphanumeric) =>
                    {
                        self.last += idx + 1;
                        self.in_chevron_link = true;
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
                    '*' | '_' => {
                        let start = &range[idx..];
                        if start.starts_with("**") || start.starts_with("__") {
                            self.last += idx + 2;
                            self.in_bold = !self.in_bold;
                            if self.in_bold {
                                yield_current!();
                            } else {
                                return Some(MarkdownTextElement::Bold(&range[..idx]));
                            }
                        } else {
                            self.last += idx + 1;
                            self.in_italic = !self.in_italic;
                            if self.in_italic {
                                yield_current!();
                            } else {
                                return Some(MarkdownTextElement::Italic(&range[..idx]));
                            }
                        }
                    }
                    _ => {}
                }
            }

            self.last = self.on.len();
            let range = if self.preserve_whitespace {
                range
            } else {
                range.trim()
            };

            if range.is_empty() {
                None
            } else if let Some(_link_text_end) = link_text_end {
                eprintln!("Link text end!!");
                None
            } else if self.in_tag {
                Some(MarkdownTextElement::Tag(range))
            } else {
                // TODO errors left overs. But also others such as tags etc
                Some(MarkdownTextElement::Plain(range))
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RawMarkdown<'a>(pub &'a str);

// #[cfg(target_family = "wasm")]
// #[wasm_bindgen]
// impl RawMarkdown<'_> {
//     #[must_use]
//     #[cfg(target_family = "wasm")]
//     #[wasm_bindgen]
//     pub fn to_html(&self, emitter: Option<crate::extras::emit::FeatureEmitterWASM>) -> String {
//         crate::extras::emit::markdown_to_html_string(&self.markdown_content, emitter)
//     }
// }

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
    pub fn cells(&self) -> impl Iterator<Item = RawText<'a>> {
        let inner = &self.0[1..(self.0.len() - 1)];
        inner.split('|').map(RawText)
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
    pub fn arguments(&self) -> Vec<(&'a str, &'a str)> {
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
            } else {
                if let '=' = chr {
                    let key_acc = &self.arguments[upto..idx];
                    key = Some(key_acc.trim());
                    upto = idx + 1;
                }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct QuoteBlock<'a> {
    /// [See GitHub markdown alerts](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts). Note this allows any alerts. It does not check from a defined list
    pub alert: Option<&'a str>,
    pub inner: &'a str,
}

fn strip_number_prefix(on: &str) -> Option<&str> {
    if on.starts_with(|chr: char| matches!(chr, '1'..'9')) {
        let level = on.chars().take_while(|c| matches!(*c, '1'..'9')).count();
        on[level..].strip_prefix([')', '.'])
    } else {
        None
    }
}

fn strip_surrounds<'a>(on: &'a str, left: &str, right: &str) -> Option<&'a str> {
    on.trim()
        .strip_prefix(left)
        .and_then(|line| line.strip_suffix(right))
        .map(str::trim)
}
