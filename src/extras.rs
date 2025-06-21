use super::{MarkdownElement, MarkdownPart, MarkdownTextElement, TextDecoration};

#[derive(Default, Clone, Copy)]
pub struct AsMarkdownOptions {
    pub uses_crlf: bool,
    pub skip_comments: bool,
}

impl MarkdownElement<'_> {
    // TODO using write
    #[must_use]
    pub fn as_markdown(&self, options: AsMarkdownOptions) -> String {
        let new_line = if options.uses_crlf { "\r\n" } else { "\n" };
        match self {
            MarkdownElement::Heading { level, content } => {
                let mut s = "#".repeat(*level as usize);
                s.push(' ');
                s.push_str(content.0);
                s
            }
            MarkdownElement::CodeBlock(crate::CodeBlock {
                language,
                code,
                indented_block: _indented_block,
            }) => {
                let mut max = 3;
                for (idx, _) in code.match_indices("```") {
                    let backticks = code[idx..]
                        .find(|chr| chr != '`')
                        .map_or(code.len(), |idx| idx + 1);
                    max = std::cmp::max(max, backticks);
                }
                let indent = &"```````````````".get(..max).expect("lol");
                let mut s = format!("{indent}{language}{new_line}");
                for line in code.lines() {
                    // Hopefully trailing whitespace has no meaning in your language
                    s.push_str(line.trim_end());
                }
                s.push_str(indent);
                s
            }
            MarkdownElement::Paragraph(content) => content.0.to_owned(),
            MarkdownElement::Quote(content) => content.inner.0.to_owned(),
            MarkdownElement::Frontmatter(frontmatter) => {
                format!("---{new_line}{source}---", source = frontmatter.0)
            }
            // #[cfg(feature = "html")]
            // MarkdownElement::HTMLElement { element: _, source } => source.to_string(),
            MarkdownElement::Empty => String::new(),
            MarkdownElement::CommentBlock(comment) => {
                if options.skip_comments {
                    String::default()
                } else {
                    format!("%%{new_line}{comment}{new_line}%%")
                }
            }
            item => format!("TODO {item:?}"),
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
        // static CONS: &str = "......................";
        // CONS.get(..self.indent).unwrap_or(CONS)

        static TABS: &str = "\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t";
        TABS.get(..self.indent).unwrap_or(TABS)

        // static SPACES: &str = "                    ";
        // SPACES.get(..(self.indent * 2)).unwrap_or(SPACES)
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
        fn from_parts(content: &crate::RawText<'_>, _options: DebugOptions) -> String {
            use std::fmt::Write;

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
                    list.parse_inner(|crate::ListItem { content, checked, enumerated  }| {
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
                        let item = if options.pretty {
                            format!("ListItem {{\n{next_indent}enumerated: {enumerated:?}, checked: {checked:?},\n{next_indent}inner: {inner}\n{indent}}}")
                        } else {
                            format!("ListItem {{ enumerated: {enumerated:?}, checked: {checked:?}, inner: {inner} }}")
                        };

                        if list_items.trim().len() > 1 {
                            list_items.push_str(if options.pretty {"," }  else { ", " });
                        }
                        if options.pretty {
                            list_items.push_str(new_line);
                            list_items.push_str(indent);
                        }
                        list_items.push_str(&item);
                    });
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
                let inner = if options.indent >= options.skip_content_after {
                    "...".to_string()
                } else {
                    let crate::RawMarkdown(content, container_residue) = *inner;
                    let mut inner = "[".to_owned();
                    if options.pretty {
                        inner.push_str(new_line);
                    }
                    // We can simplify the output if it is just a paragraph
                    let mut just_paragraph = None;
                    let parse_options = crate::ParseOptions::default();

                    let _result = crate::parse_with_options::<()>(
                        content,
                        parse_options,
                        container_residue,
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
                    let next_indent = options.next().get_indent();
                    if let Some(just_paragraph) = just_paragraph {
                        from_parts(&just_paragraph, options)
                    } else {
                        if options.pretty {
                            inner.push_str(new_line);
                            inner.push_str(next_indent);
                        }
                        inner.push(']');
                        inner
                    }
                };

                #[allow(clippy::collapsible_else_if)]
                if options.pretty {
                    let next_indent = options.next().get_indent();
                    if let Some(alert) = alert {
                        format!("{indent}QuoteBlock {{\n{next_indent}alert: {alert:?},\n{next_indent}inner: {inner}\n{indent}}}")
                    } else {
                        format!("{indent}QuoteBlock {{\n{next_indent}inner: {inner}\n{indent}}}")
                    }
                } else {
                    if let Some(alert) = alert {
                        format!("{indent}QuoteBlock {{ alert: {alert:?}, inner: {inner} }}")
                    } else {
                        format!("{indent}QuoteBlock {{ inner: {inner} }}")
                    }
                }
            }
            MarkdownElement::CodeBlock(crate::CodeBlock {
                language,
                code,
                indented_block,
            }) => {
                let code = if options.indent >= options.skip_content_after {
                    "..."
                } else {
                    code
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
            #[cfg(feature = "html")]
            MarkdownElement::HTMLElement { element, .. } => {
                todo!()
                // format!("HTMLElement({element:?})")
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
            // rest
            item => format!("{indent}{item:?}"),
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
        for part in crate::PartsIterator::new(self.0, self.1) {
            if let crate::MarkdownPart::Plain = part.kind {
                s.push_str(part.on);
            }
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
