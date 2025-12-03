use super::{MarkdownElement, MarkdownPart, MarkdownTextElement, TextDecoration};
use std::io::Write;

#[derive(Default, Clone, Copy)]
pub struct AsMarkdownOptions {
    pub uses_crlf: bool,
    pub skip_comments: bool,
}

impl MarkdownElement<'_> {
    #[allow(clippy::too_many_lines)]
    pub fn as_markdown(
        &self,
        out: &mut impl Write,
        indent: &str,
        options: AsMarkdownOptions,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let new_line = if options.uses_crlf { "\r\n" } else { "\n" };
        match self {
            MarkdownElement::Heading { level, content } => {
                let level = *level as usize;
                let prefix = "######"
                    .get(..level)
                    .expect("can only print tags upto 6 levels");
                // TODO content
                write!(out, "{indent}{prefix} {content}", content = content.0)?;
                Ok(())
            }
            MarkdownElement::CodeBlock(crate::CodeBlock {
                language,
                // TODO
                raw_code: code,
                indented_block: _indented_block,
                container_residue: _,
            }) => {
                let mut max = 3;
                // Figure out depth
                for (idx, _) in code.match_indices("```") {
                    let backticks = code[idx..]
                        .find(|chr| chr != '`')
                        .map_or(code.len(), |idx| idx + 1);
                    max = std::cmp::max(max, backticks);
                }
                let fence = &"```````````````".get(..max).expect("lol");
                write!(out, "{indent}{fence}{language}{new_line}")?;
                for line in code.lines() {
                    // Hopefully trailing whitespace has no meaning in your language
                    write!(
                        out,
                        "{indent}{content}{new_line}",
                        content = line.trim_end()
                    )?;
                }
                write!(out, "{indent}{fence}")?;
                Ok(())
            }
            MarkdownElement::Paragraph(content) => {
                // TODO content processing
                write!(out, "{indent}{content}", content = content.0)?;
                Ok(())
            }
            MarkdownElement::Quote(content) => {
                write!(out, "{indent}{content}", content = content.inner.0)?;
                Ok(())
            }
            MarkdownElement::Frontmatter(frontmatter) => {
                // TODO formatting using `simple_yaml_parser`
                write!(out, "---{new_line}{source}---", source = frontmatter.0)?;
                Ok(())
            }
            // #[cfg(feature = "html")]
            // MarkdownElement::HTMLElement { element: _, source } => source.to_string(),
            MarkdownElement::CommentBlock(comment) => {
                if !options.skip_comments {
                    write!(out, "%%{new_line}{comment}{new_line}%%")?;
                }
                Ok(())
            }
            MarkdownElement::List(list) => {
                let mut multiple_items = false;
                list.parse_inner::<Box<dyn std::error::Error>>(|list_item| {
                    let crate::ListItem {
                        content,
                        checked: _,
                        enumerated: _,
                    } = list_item;
                    if multiple_items {
                        write!(out, "{new_line}")?;
                    }
                    // TODO checked & enumerated
                    let list_indent = "  ";
                    write!(out, "{indent}- ")?;
                    let new_indent = format!("{indent}{list_indent}");
                    let mut multiple = false;
                    let parse_options = crate::ParseOptions::default();
                    crate::parse_with_options::<Box<dyn std::error::Error>>(
                        content.0,
                        parse_options,
                        content.1,
                        |item| {
                            let indent = if multiple {
                                write!(out, "{new_line}")?;
                                &new_indent
                            } else {
                                indent
                            };
                            item.as_markdown(out, indent, options)?;
                            multiple = true;
                            Ok(())
                        },
                    )?;
                    multiple_items = true;
                    Ok(())
                })
            }
            MarkdownElement::Table(table) => {
                // TODO
                write!(out, "{content}", content = table.0)?;
                Ok(())
            }
            MarkdownElement::MathematicsBlock(content) => {
                // TODO
                write!(out, "$$\n{content}\n$$", content = content.0)?;
                Ok(())
            }
            MarkdownElement::CommandBlock(_command) => {
                todo!()
                // TODO
                // write!(out, "$$\n{command}\n$$", command=command.0)?;
            }
            MarkdownElement::HTMLElement(_element) => todo!(),
            MarkdownElement::HorizontalRule => todo!(),
            MarkdownElement::Empty => Ok(()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct DebugOptions {
    pub indent: usize,
    pub skip_content_after: usize,
    pub pretty: bool,
}

impl DebugOptions {
    pub(crate) fn next(self) -> Self {
        Self {
            indent: self.indent + 1,
            skip_content_after: self.skip_content_after,
            pretty: self.pretty,
        }
    }

    pub(crate) fn get_indent(self) -> &'static str {
        // static SPACES: &str = "                        ";
        // SPACES.get(..(self.indent * 2)).unwrap_or(SPACES)
        static TABS: &str = "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t";
        TABS.get(..self.indent).unwrap_or(TABS)
    }
}

impl Default for DebugOptions {
    fn default() -> Self {
        Self {
            indent: 0,
            skip_content_after: usize::MAX,
            pretty: true,
        }
    }
}

impl MarkdownElement<'_> {
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn debug_with_options(&self, options: DebugOptions) -> String {
        use std::fmt::Write;

        fn from_parts(content: &crate::RawText<'_>, _options: DebugOptions) -> String {
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
                    if part.decoration != TextDecoration::NONE {
                        // TODO as set or something?
                        if part.decoration.contains(TextDecoration::BOLD)
                            && part.decoration.contains(TextDecoration::EMPHASIS)
                        {
                            s.push_str("bold & emphasised: ");
                        } else if part.decoration.contains(TextDecoration::BOLD) {
                            s.push_str("bold: ");
                        } else if part.decoration.contains(TextDecoration::EMPHASIS) {
                            s.push_str("emphasised: ");
                        }
                        if part.decoration.contains(TextDecoration::HIGHLIGHTED) {
                            s.push_str("highlighted: ");
                        }
                        if part.decoration.contains(TextDecoration::STRIKETHROUGH) {
                            s.push_str("strikethrough: ");
                        }
                        if part.decoration.contains(TextDecoration::SUPERSCRIPT) {
                            s.push_str("superscript: ");
                        }
                        if part.decoration.contains(TextDecoration::SUBSCRIPT) {
                            s.push_str("subscript: ");
                        }
                    }
                    write!(&mut s, "{:?}", part.on).unwrap();
                    s.push(')');
                }
                s.push(']');
                s
            }
        }

        let indent = options.get_indent();
        let new_line = if options.pretty { "\n" } else { "" };

        match self {
            MarkdownElement::Heading { level, content } => {
                let inner = if options.indent >= options.skip_content_after {
                    "...".to_owned()
                } else {
                    from_parts(content, options)
                };
                format!("{indent}Heading {{ level: {level}, content: {inner} }}")
            }
            MarkdownElement::Paragraph(content) => {
                let inner = if options.indent >= options.skip_content_after {
                    "...".to_owned()
                } else {
                    from_parts(content, options)
                };
                format!("{indent}Paragraph({inner})")
            }
            MarkdownElement::List(list) => {
                let inner = if options.indent >= options.skip_content_after {
                    "...".to_owned()
                } else {
                    let mut list_items = "[".to_owned();

                    // FUTURE pass options down?
                    let parse_options = crate::ParseOptions::default();
                    list.parse_inner::<()>(
                        |crate::ListItem {
                             content,
                             checked,
                             enumerated,
                         }| {
                            let options = options.next();
                            let mut inner = "[".to_owned();
                            if options.pretty {
                                inner.push_str(new_line);
                            }
                            // We can simplify the output if it is just a paragraph
                            let mut just_paragraph = None;

                            let _result = crate::parse_with_options::<()>(
                                content.0,
                                parse_options,
                                content.1,
                                |item| {
                                    if inner.trim_end().len() > 1 {
                                        let _ = just_paragraph.take();
                                        if options.pretty {
                                            inner.push(',');
                                            inner.push_str(new_line);
                                        } else {
                                            inner.push_str(", ");
                                        }
                                    } else if let MarkdownElement::Paragraph(item) = item {
                                        just_paragraph = Some(item);
                                    }
                                    inner.push_str(&item.debug_with_options(options.next().next()));
                                    Ok(())
                                },
                            );
                            let indent = options.get_indent();
                            let next_indent = options.next().get_indent();

                            let inner = if let Some(just_paragraph) = just_paragraph {
                                from_parts(&just_paragraph, options)
                            } else {
                                if options.pretty {
                                    inner.push_str(new_line);
                                    inner.push_str(next_indent);
                                }
                                inner.push(']');
                                inner
                            };

                            if list_items.trim().len() > 1 {
                                list_items.push_str(if options.pretty { "," } else { ", " });
                            }

                            if options.pretty {
                                list_items.push_str(new_line);
                                list_items.push_str(indent);
                            }

                            if options.pretty {
                                writeln!(&mut list_items, "ListItem {{").unwrap();
                                if enumerated {
                                    writeln!(&mut list_items, "{next_indent}enumerated: true,")
                                        .unwrap();
                                }
                                if checked.is_some() {
                                    writeln!(&mut list_items, "{next_indent}checked: {checked:?},")
                                        .unwrap();
                                }
                                writeln!(&mut list_items, "{next_indent}inner: {inner}").unwrap();
                                write!(&mut list_items, "{indent}}}").unwrap();
                            } else {
                                write!(&mut list_items, "ListItem {{").unwrap();
                                if enumerated {
                                    write!(&mut list_items, " enumerated: true,").unwrap();
                                }
                                if checked.is_some() {
                                    write!(&mut list_items, " checked: {checked:?},").unwrap();
                                }
                                write!(&mut list_items, " inner: {inner}").unwrap();
                                write!(&mut list_items, "}}").unwrap();
                            }
                            Ok(())
                        },
                    )
                    .unwrap();

                    if options.pretty {
                        list_items.push_str(new_line);
                        list_items.push_str(options.get_indent());
                    }
                    list_items.push(']');
                    list_items
                };
                format!("{indent}List({inner})")
            }
            MarkdownElement::Quote(crate::QuoteBlock { alert, inner }) => {
                let inner_buf = if options.indent >= options.skip_content_after {
                    "...".to_string()
                } else {
                    let crate::RawMarkdown(inner, container_residue) = *inner;
                    let mut inner_buf = "[".to_owned();
                    if options.pretty {
                        inner_buf.push_str(new_line);
                    }
                    // We can simplify the output if it is just a paragraph
                    let mut just_paragraph = None;
                    let parse_options = crate::ParseOptions::default();

                    let _result = crate::parse_with_options::<()>(
                        &inner,
                        parse_options,
                        container_residue,
                        |item| {
                            if inner_buf.trim_end().len() > 1 {
                                let _ = just_paragraph.take();
                                if options.pretty {
                                    inner_buf.push(',');
                                    inner_buf.push_str(new_line);
                                } else {
                                    inner_buf.push_str(", ");
                                }
                            } else if let MarkdownElement::Paragraph(item) = item {
                                just_paragraph = Some(item);
                            }
                            inner_buf.push_str(&item.debug_with_options(options.next().next()));
                            Ok(())
                        },
                    );
                    let next_indent = options.next().get_indent();
                    if let Some(just_paragraph) = just_paragraph {
                        from_parts(&just_paragraph, options)
                    } else {
                        if options.pretty {
                            inner_buf.push_str(new_line);
                            inner_buf.push_str(next_indent);
                        }
                        inner_buf.push(']');
                        inner_buf
                    }
                };

                #[allow(clippy::collapsible_else_if)]
                if options.pretty {
                    let next_indent = options.next().get_indent();
                    if let Some(alert) = alert {
                        format!("{indent}QuoteBlock {{\n{next_indent}alert: {alert:?},\n{next_indent}inner: {inner_buf}\n{indent}}}")
                    } else {
                        format!(
                            "{indent}QuoteBlock {{\n{next_indent}inner: {inner_buf}\n{indent}}}"
                        )
                    }
                } else {
                    if let Some(alert) = alert {
                        format!("{indent}QuoteBlock {{ alert: {alert:?}, inner: {inner_buf} }}")
                    } else {
                        format!("{indent}QuoteBlock {{ inner: {inner_buf} }}")
                    }
                }
            }
            MarkdownElement::CodeBlock(crate::CodeBlock {
                language,
                raw_code,
                indented_block,
                container_residue: _,
            }) => {
                // TODO more
                let code = if options.indent >= options.skip_content_after {
                    "..."
                } else {
                    raw_code
                };
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
            MarkdownElement::Table(table) => {
                // TODO header?
                // TODO pretty
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
                        source.push_str(&from_parts(&cell, options));
                    }
                    source.push(']');
                }
                source.push(']');
                format!("{indent}Table({source})")
            }
            MarkdownElement::CommandBlock(command_block) => {
                let inner = if options.indent >= options.skip_content_after {
                    "...".to_string()
                } else {
                    let mut s = "[".to_owned();
                    // FUTURE pass options down?
                    let parse_options = crate::ParseOptions::default();
                    let _result = crate::parse_with_options::<()>(
                        command_block.inner.0,
                        parse_options,
                        command_block.inner.1,
                        |item| {
                            if s.len() > 1 {
                                s.push_str(", ");
                            }
                            s.push_str(&item.debug_with_options(options));
                            Ok(())
                        },
                    );
                    s.push(']');
                    s
                };
                format!(
                    "{indent}CommandBlock {{ name: {name}, arguments: {arguments:?}, inner: {inner} }}",
                    name = command_block.name,
                    arguments = command_block.parse_arguments()
                )
            }
            MarkdownElement::HTMLElement(element) => {
                #[cfg(feature = "html")]
                todo!("debug element");

                #[cfg(not(feature = "html"))]
                format!("HTMLElement({element:?})")
            }
            #[cfg(feature = "yaml")]
            MarkdownElement::Frontmatter(frontmatter) => {
                use std::fmt::Write;

                let mut s = indent.to_owned();
                s.push_str("Frontmatter { ");
                let _ = frontmatter.parse_yaml(|key, value| {
                    if s.ends_with(')') {
                        s.push_str(", ");
                    }
                    write!(&mut s, "{key:?} -> {value:?}").unwrap();
                });
                s.push_str(" }");
                s
            }
            item @ (MarkdownElement::MathematicsBlock(_)
            | MarkdownElement::CommentBlock(_)
            | MarkdownElement::HorizontalRule
            | MarkdownElement::Empty) => format!("{item:?}"),
        }
    }
}

impl<'a> MarkdownElement<'a> {
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
    pub fn parts_like(&self) -> Option<crate::RawText<'a>> {
        if let MarkdownElement::Heading { content, .. } | MarkdownElement::Paragraph(content) = self
        {
            Some(*content)
        } else {
            // no quote or list item here
            None
        }
    }
}

impl crate::RawText<'_> {
    #[must_use]
    pub fn no_decoration(&self) -> String {
        let mut s = String::new();
        for part in crate::PartsIterator::new_with_container_residue(self.0, self.1) {
            // if let crate::MarkdownPart::Plain = part.kind {
            s.push_str(part.on);
            // }
        }
        s
    }
}

impl crate::List<'_> {
    #[must_use]
    pub fn is_ordered(&self) -> bool {
        let is_unordered = self
            .0
             .1
            .strip_prefix(self.0 .0)
            .starts_with(super::parser::EXTENDED_LIST_PREFIXES);
        !is_unordered
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

impl<'a> crate::CodeBlock<'a> {
    /// Prefer [`CodeBlock::content_lines`]
    pub fn content(&self) -> std::borrow::Cow<'a, str> {
        if self.container_residue.is_empty() {
            std::borrow::Cow::Borrowed(self.raw_code)
        } else {
            let mut buf = String::new();
            for line in self.content_lines() {
                buf.push_str(line);
                // TODO custom?
                buf.push('\n');
            }
            std::borrow::Cow::Owned(buf)
        }
    }

    pub fn content_lines(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.raw_code
            .lines()
            .map(|line| self.container_residue.strip_prefix(line))
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
