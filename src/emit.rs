use crate::{
    Frontmatter, MarkdownElement, MarkdownParseError, MarkdownPart, MarkdownTextElement,
    ParseOptions, RawText, TextDecoration,
};
use std::fmt::Write;

// TODO
// #[cfg(target_family = "wasm")]
// mod wasm;

pub fn markdown_to_html(
    source: &str,
    emitter: &mut impl FeatureEmitter,
    options: ParseOptions,
) -> Result<(), MarkdownParseError<()>> {
    crate::parse_with_options::<()>(
        source,
        options,
        crate::ContainerResidue::default(),
        |item| {
            element_to_html(emitter, options, item).unwrap();
            Ok(())
        },
    )
}

pub trait FeatureEmitter: std::fmt::Write {
    fn frontmatter(&mut self, content: Frontmatter<'_>);

    fn code_block(&mut self, language: &str, code: &str);

    fn mathematics(&mut self, code: &str, display: bool);

    fn command(&mut self, name: &str, args: Vec<(&str, &str)>, inner: &str, options: ParseOptions);

    fn interpolation(&mut self, expression: &str);
}

/// Un-highlighted code and panics on `RegExp`
pub struct BlankFeatureEmitter<T>(T);

impl<T> std::fmt::Write for BlankFeatureEmitter<T>
where
    T: std::fmt::Write,
{
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        T::write_str(&mut self.0, s)
    }
}

impl<T> FeatureEmitter for BlankFeatureEmitter<T>
where
    T: std::fmt::Write,
{
    fn frontmatter(&mut self, _content: Frontmatter<'_>) {
        eprintln!("skipping frontmatter");
    }

    fn code_block(&mut self, _language: &str, _code: &str) {
        panic!("`BlankFeatureEmitter` does implement code blocks");
        // code.to_owned()
    }

    fn mathematics(&mut self, _content: &str, _display: bool) {
        panic!("`BlankFeatureEmitter` does implement LaTeX HTML generation");
        // if display {
        //     format!("$${content}$$")
        // } else {
        //     format!("${content}$")
        // }
    }

    fn command(
        &mut self,
        _name: &str,
        _args: Vec<(&str, &str)>,
        _inner: &str,
        _options: ParseOptions,
    ) {
        panic!("`BlankFeatureEmitter` does implement command generation")
    }

    fn interpolation(&mut self, _expression: &str) {
        panic!("`BlankFeatureEmitter` does implement interpolation")
    }
}

#[allow(clippy::too_many_lines)]
pub fn element_to_html(
    emitter: &mut impl FeatureEmitter,
    options: ParseOptions,
    item: MarkdownElement,
) -> Result<(), Box<dyn std::error::Error>> {
    match item {
        MarkdownElement::Heading { level, content } => {
            assert!(level < 7, "heading level too much for HTML");
            writeln!(emitter, "<h{level}>")?;
            inner_to_html(emitter, content)?;
            writeln!(emitter, "</h{level}>")?;
        }
        MarkdownElement::Quote(block) => {
            writeln!(emitter, "<blockquote")?;
            if let Some(alert) = block.alert {
                // TODO
                write!(
                    emitter,
                    " data-quote-alert=\"{alert}\">",
                    alert = alert.to_lowercase()
                )
                .unwrap();
            } else {
                write!(emitter, ">")?;
            }
            let crate::RawMarkdown(inner, container_residue) = block.inner;
            let _ = crate::parse_with_options(inner, options, container_residue, |item| {
                element_to_html(emitter, options, item)
            });
            writeln!(emitter, "</blockquote>")?;
        }
        MarkdownElement::Paragraph(content) => {
            if content.0.starts_with("![") || content.0.starts_with("[![") {
                // Don't wrap media in `<p>`
                inner_to_html(emitter, content)?;
            } else {
                writeln!(emitter, "<p>")?;
                inner_to_html(emitter, content)?;
                writeln!(emitter, "</p>")?;
            }
        }
        MarkdownElement::List(list) => {
            let tag_name = if list.is_ordered() { "ol" } else { "ul" };
            writeln!(emitter, "<{tag_name}>")?;
            list.parse_inner::<Box<dyn std::error::Error>>(|item| {
                writeln!(emitter, "<li>")?;
                let crate::RawMarkdown(inner, container_residue) = item.content;
                crate::parse_with_options(inner, options, container_residue, |item| {
                    element_to_html(emitter, options, item)
                })?;
                writeln!(emitter, "</li>")?;
                Ok(())
            })?;
            writeln!(emitter, "</{tag_name}>")?;
        }
        MarkdownElement::Table(table) => {
            writeln!(emitter, "<table>")?;
            let mut rows = table.rows();
            writeln!(emitter, "<thead><tr>")?;
            for cell in rows.next().unwrap().cells() {
                write!(emitter, "<th>")?;
                inner_to_html(emitter, cell)?;
                writeln!(emitter, "</th>")?;
            }
            writeln!(emitter, "</tr></thead>")?;
            writeln!(emitter, "<tbody>")?;
            for row in rows {
                write!(emitter, "<tr>")?;
                for cell in row.cells() {
                    write!(emitter, "<td>")?;
                    inner_to_html(emitter, cell)?;
                    write!(emitter, "</td>")?;
                }
                writeln!(emitter, "</tr>")?;
            }
            writeln!(emitter, "</tbody>")?;
            writeln!(emitter, "</table>")?;
        }
        MarkdownElement::CodeBlock(
            block @ crate::CodeBlock {
                language,
                indented_block: _,
                ..
            },
        ) => {
            let code = block.content();
            emitter.code_block(language, &code);
        }
        MarkdownElement::MathematicsBlock(crate::MathematicsBlock(script)) => {
            emitter.mathematics(script, true);
        }
        MarkdownElement::HorizontalRule => {
            writeln!(emitter, "<hr>")?;
        }
        MarkdownElement::CommandBlock(command) => {
            emitter.command(
                command.name,
                command.parse_arguments(),
                command.inner.0,
                options,
            );
        }
        MarkdownElement::HTMLElement(html) => {
            writeln!(emitter, "{content}", content = html.0)?;
        }
        // writeln!(emitter, "<pre class=\"frontmatter\">{}</pre>", inner.0)?;
        MarkdownElement::Frontmatter(frontmatter) => emitter.frontmatter(frontmatter),
        MarkdownElement::CommentBlock(_) | MarkdownElement::Empty => {}
    }

    // #[cfg(feature = "html")]
    // MarkdownElement::HTMLElement { source: _, element } => {
    //     fn emit_element(
    //         element: &lightml::Element<'_>,
    //         out: &mut impl Write,
    //         emitter: &impl FeatureEmitter,
    //         options: ParseOptions,
    //     ) -> Result<(), Box<dyn std::error::Error>> {
    //         write!(emitter, "<{tag_name}", tag_name = element.tag_name)?;
    //         for lightml::Attribute { key, value } in &element.attributes {
    //             write!(emitter, " \"{key}\"=\"{value}\"")?;
    //         }
    //         writeln!(emitter, ">")?;
    //         match element.children {
    //             lightml::ElementChildren::Children(ref children) => {
    //                 for child in children {
    //                     match child {
    //                         lightml::Node::Element(element) => {
    //                             let _ = emit_element(&element, out, emitter, options)?;
    //                         }
    //                         lightml::Node::TextNode(content) => {
    //                             // Yes it is mapped recursively
    //                             // TODO unwrap
    //                             let _ = markdown_to_html(content, out, emitter, options, 0)
    //                                 .unwrap();
    //                         }
    //                         lightml::Node::Comment(_)
    //                         | lightml::Node::MismatchClosingTag(_) => {}
    //                     }
    //                 }
    //                 writeln!(emitter, "</{tag_name}>", tag_name = element.tag_name)?;
    //             }
    //             lightml::ElementChildren::SelfClosing => {}
    //             lightml::ElementChildren::Literal(ref content) => {
    //                 writeln!(emitter, "{content}\n</{tag_name}>", tag_name = element.tag_name)?;
    //             }
    //         }
    //         Ok(())
    //     }

    //     let _ = emit_element(&element, out, emitter, options)?;
    // }

    Ok(())
}

pub fn inner_to_html(
    emitter: &mut impl FeatureEmitter,
    content: RawText,
) -> Result<(), Box<dyn std::error::Error>> {
    fn update_decoration(
        emitter: &mut impl Write,
        opening: TextDecoration,
        closing: TextDecoration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if opening.contains(TextDecoration::BOLD) {
            write!(emitter, "<bold>")?;
        } else if closing.contains(TextDecoration::BOLD) {
            write!(emitter, "</bold>")?;
        }
        if opening.contains(TextDecoration::EMPHASIS) {
            write!(emitter, "<em>")?;
        } else if closing.contains(TextDecoration::EMPHASIS) {
            write!(emitter, "</em>")?;
        }
        if opening.contains(TextDecoration::HIGHLIGHTED) {
            // FUTURE change default
            write!(emitter, "<span style=\"background-color: yellow\">")?;
        } else if closing.contains(TextDecoration::HIGHLIGHTED) {
            write!(emitter, "</span>")?;
        }
        if opening.contains(TextDecoration::STRIKETHROUGH) {
            write!(emitter, "<s>")?;
        } else if closing.contains(TextDecoration::STRIKETHROUGH) {
            write!(emitter, "</s>")?;
        }
        if opening.contains(TextDecoration::SUBSCRIPT) {
            write!(emitter, "<sub>")?;
        } else if closing.contains(TextDecoration::SUBSCRIPT) {
            write!(emitter, "</sub>")?;
        }
        if opening.contains(TextDecoration::SUPERSCRIPT) {
            write!(emitter, "<sup>")?;
        } else if closing.contains(TextDecoration::SUPERSCRIPT) {
            write!(emitter, "</sup>")?;
        }
        Ok(())
    }

    let mut current = TextDecoration::NONE;
    for MarkdownTextElement {
        on,
        decoration,
        kind,
    } in content.parts()
    {
        let difference = current ^ decoration;
        update_decoration(emitter, difference & decoration, difference & current)?;
        match kind {
            MarkdownPart::Plain => write!(emitter, "{on}", on = escape_string_content(on))?,
            MarkdownPart::InlineCode => {
                write!(emitter, "<code>{on}</code>", on = escape_string_content(on))?;
            }
            MarkdownPart::InlineMathematics => emitter.mathematics(on, false),
            MarkdownPart::Emoji => write!(emitter, "")?,
            MarkdownPart::Tag => write!(emitter, "")?,
            MarkdownPart::Interpolation => write!(emitter, "")?,
            MarkdownPart::RawLink => write!(emitter, "<a href=\"{on}\">{on}</a>")?,
            MarkdownPart::ExternalLink { to } => write!(
                emitter,
                "<a href=\"{to}\">{on}</a>",
                on = escape_string_content(on)
            )?,
            MarkdownPart::MediaLink { source, alt } => write!(
                emitter,
                "<img src=\"{source}\" alt=\"{alt}\">",
                alt = escape_string_content(alt)
            )?,
            MarkdownPart::LineBreak => write!(emitter, "<br>",)?,
            // FUTURE improve
            MarkdownPart::InternalLink { to } => write!(
                emitter,
                "<a href=\"#{to}\">{on}</a>",
                on = escape_string_content(on)
            )?,
            MarkdownPart::HTMLElement(element) => {
                write!(emitter, "{content}", content = element.0)?
            }
        }
        current = decoration;
    }
    update_decoration(emitter, TextDecoration::NONE, current)?;
    Ok(())
}

#[must_use]
pub fn escape_string_content(on: &str) -> std::borrow::Cow<'_, str> {
    let mut result = std::borrow::Cow::Borrowed("");
    let mut start = 0;
    for (index, matched) in on.match_indices(['<', '>', '"', '&']) {
        result += &on[start..index];
        result += match matched {
            "\"" => "&quot;",
            "&" => "&amp;",
            "<" => "&lt;",
            ">" => "&gt;",
            _ => unreachable!(),
        };
        start = index + matched.len();
    }
    result += &on[start..];
    result
}
