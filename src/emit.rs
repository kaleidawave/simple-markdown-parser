use crate::{
    MarkdownElement, MarkdownParseError, MarkdownPart, MarkdownTextElement, ParseOptions, RawText,
    TextDecoration,
};
use std::io::Write;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub fn markdown_to_html_string(source: &str, emitter: Option<FeatureEmitterWASM>) -> String {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let mut bytes: Vec<u8> = Vec::new();
    // TODO as parameter
    let options = ParseOptions::default();
    let _ = match emitter {
        Some(mut emitter) => markdown_to_html(source, &mut bytes, &mut emitter, options),
        None => markdown_to_html(source, &mut bytes, &mut BlankFeatureEmitter, options),
    };
    match String::from_utf8(bytes) {
        Ok(result) => result,
        Err(_) => String::from("Non Utf8 output or markdown parser error"),
    }
}

pub fn markdown_to_html(
    source: &str,
    _out: &mut impl Write,
    _emitter: &impl FeatureEmitter,
    options: ParseOptions,
    container_residue: super::ContainerResidue<'_>,
) -> Result<(), MarkdownParseError<()>> {
    // let mut last_was_list_item: Option<&'static str> = None;
    crate::parse_with_options::<()>(source, options, container_residue, |_item| {
        todo!();
        // let is_list_item = if let MarkdownElement::ListItem {
        //     enumerated,
        //     checked: _,
        //     ..
        // } = item
        // {
        //     Some(if enumerated { "ol" } else { "ul" })
        // } else {
        //     None
        // };
        // if let (Some(tag), None) = (is_list_item, last_was_list_item) {
        //     writeln!(out, "<{tag}>").unwrap();
        // } else if let (None, Some(tag)) = (is_list_item, last_was_list_item) {
        //     writeln!(out, "</{tag}>").unwrap();
        // }
        // element_to_html(out, emitter, options, quote_depth, item).unwrap();
        // last_was_list_item = is_list_item;

        Ok(())
    })
}

pub trait FeatureEmitter {
    fn code_block(&self, language: &str, code: &str) -> String;

    fn mathematics(&self, code: &str, display: bool) -> String;

    fn command(
        &self,
        name: &str,
        args: Vec<(&str, &str)>,
        inner: &str,
        options: ParseOptions,
        to: &mut impl Write,
    );

    fn interpolation(&self, expression: &str) -> String;
}

/// Un-highlighted code and panics on `RegExp`
pub struct BlankFeatureEmitter;

impl FeatureEmitter for BlankFeatureEmitter {
    fn code_block(&self, _language: &str, code: &str) -> String {
        // panic!("`BlankFeatureEmitter` does implement code blocks");
        code.to_owned()
    }

    fn mathematics(&self, content: &str, display: bool) -> String {
        // panic!("`BlankFeatureEmitter` does implement LaTeX HTML generation");
        if display {
            format!("$${content}$$")
        } else {
            format!("${content}$")
        }
    }

    fn command(
        &self,
        _name: &str,
        _args: Vec<(&str, &str)>,
        _inner: &str,
        _options: ParseOptions,
        _to: &mut impl Write,
    ) {
        panic!("`BlankFeatureEmitter` does implement command generation")
    }

    fn interpolation(&self, _expression: &str) -> String {
        panic!("`BlankFeatureEmitter` does implement interpolation")
    }
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen(skip_typescript)]
pub struct FeatureEmitterWASM {
    code_block_callback: js_sys::Function,
    mathematics_callback: js_sys::Function,
    command_callback: js_sys::Function,
    interpolation_callback: js_sys::Function,
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export class FeatureEmitterWASM { 
    constructor(
        code_block_callback: (language: string, code: string) => string,
        mathematics_callback: (code: string) => string,
        command_callback: (name: string, args: Array<[string, string]>, inner: string) => string,
        interpolation_callback: (expression: string) => string,
    );
}
"#;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
impl FeatureEmitterWASM {
    #[wasm_bindgen(constructor)]
    pub fn new(
        code_block_callback: js_sys::Function,
        mathematics_callback: js_sys::Function,
        command_callback: js_sys::Function,
        interpolation_callback: js_sys::Function,
    ) -> Self {
        Self {
            code_block_callback,
            mathematics_callback,
            command_callback,
            interpolation_callback,
        }
    }
}

#[cfg(target_family = "wasm")]
fn result_to_string(result: Result<JsValue, JsValue>) -> String {
    result
        .ok()
        .as_ref()
        .and_then(JsValue::as_string)
        .unwrap_or_else(|| "Error".to_owned())
}

#[cfg(target_family = "wasm")]
impl FeatureEmitter for FeatureEmitterWASM {
    fn code_block(&self, language: &str, code: &str) -> String {
        let result = self.code_block_callback.call2(
            &JsValue::NULL,
            &JsValue::from_str(language),
            &JsValue::from_str(code),
        );
        result_to_string(result)
    }

    fn mathematics(&self, code: &str, display: bool) -> String {
        let result = self.mathematics_callback.call1(
            &JsValue::NULL,
            &JsValue::from_str(code),
            &JsValue::from_bool(display),
        );
        result_to_string(result)
    }

    fn command(
        &self,
        name: &str,
        args: Vec<(&str, &str)>,
        inner: &str,
        _options: ParseOptions,
        to: &mut impl Write,
    ) {
        use js_sys::Array;

        let args_array = Array::new();
        args.into_iter().for_each(|(l, r)| {
            args_array.push(&Array::of2(&JsValue::from_str(l), &JsValue::from_str(r)).into());
        });
        let result = self.command_callback.call3(
            &JsValue::NULL,
            &JsValue::from_str(name),
            &args_array.into(),
            &JsValue::from_str(inner),
        );
        write!(to, "{}", result_to_string(result));
    }

    fn interpolation(&self, expression: &str) -> String {
        let result = self
            .interpolation_callback
            .call1(&JsValue::NULL, &JsValue::from_str(expression));
        result_to_string(result)
    }
}

#[allow(clippy::match_same_arms)]
pub fn element_to_html(
    out: &mut impl Write,
    emitter: &impl FeatureEmitter,
    options: ParseOptions,
    _quote_depth: u8,
    item: MarkdownElement,
) -> Result<(), Box<dyn std::error::Error>> {
    match item {
        MarkdownElement::Heading { level, content } => {
            assert!(level < 7, "heading level too much for HTML");
            writeln!(out, "<h{level}>")?;
            inner_to_html(out, emitter, content)?;
            writeln!(out, "</h{level}>")?;
        }
        MarkdownElement::Quote(block) => {
            writeln!(out, "<blockquote")?;
            if let Some(alert) = block.alert {
                // TODO
                write!(
                    out,
                    " data-quote-alert=\"{alert}\">",
                    alert = alert.to_lowercase()
                )
                .unwrap();
            } else {
                write!(out, ">")?;
            }
            todo!();
            // markdown_to_html(block.inner, out, emitter, options, quote_depth + 1).unwrap();
            writeln!(out, "</blockquote>")?;
        }
        MarkdownElement::Paragraph(content) => {
            if content.0.starts_with("![") || content.0.starts_with("[![") {
                // Don't wrap media in `<p>`
                inner_to_html(out, emitter, content)?;
            } else {
                writeln!(out, "<p>")?;
                inner_to_html(out, emitter, content)?;
                writeln!(out, "</p>")?;
            }
        }
        MarkdownElement::List(_list) => {
            todo!()
        }
        // MarkdownElement::ListItem {
        //     level: _level,
        //     content,
        //     enumerated: _,
        //     checked: _,
        // } => {
        //     writeln!(out, "<li>")?;
        //     inner_to_html(out, emitter, content)?;
        //     writeln!(out, "</li>")?;
        // }
        // TODO test
        MarkdownElement::Table(table) => {
            writeln!(out, "<table>")?;
            let mut rows = table.rows();
            writeln!(out, "<thead><tr>")?;
            for cell in rows.next().unwrap().cells() {
                write!(out, "<th>")?;
                inner_to_html(out, emitter, cell)?;
                writeln!(out, "</th>")?;
            }
            writeln!(out, "</tr></thead>")?;
            writeln!(out, "<tbody>")?;
            for row in rows {
                write!(out, "<tr>")?;
                for cell in row.cells() {
                    write!(out, "<td>")?;
                    inner_to_html(out, emitter, cell)?;
                    write!(out, "</td>")?;
                }
                writeln!(out, "</tr>")?;
            }
            writeln!(out, "</tbody>")?;
            writeln!(out, "</table>")?;
        }
        MarkdownElement::CodeBlock(crate::CodeBlock {
            language,
            code,
            indented_block: _,
        }) => {
            let inner = emitter.code_block(language, code);
            writeln!(
                out,
                "<pre data-language=\"{language}\"><code>{inner}</code></pre>"
            )?;
        }
        MarkdownElement::BlockMathematics(crate::BlockMathematics(script)) => {
            writeln!(
                out,
                "<p class=\"mathematics block\">{inner}</p>",
                inner = emitter.mathematics(script, true)
            )?;
        }
        MarkdownElement::HorizontalRule => {
            writeln!(out, "<hr>")?;
        }
        MarkdownElement::CommandBlock(command) => {
            emitter.command(
                command.name,
                command.parse_arguments(),
                command.inner.0,
                options,
                out,
            );
        }
        MarkdownElement::HTMLElement(html) => {
            writeln!(out, "{content}", content = html.0)?;
        }
        // writeln!(out, "<pre class=\"frontmatter\">{}</pre>", inner.0)?;
        MarkdownElement::Frontmatter(_)
        | MarkdownElement::CommentBlock(_)
        | MarkdownElement::Empty => {}
    }
    // #[cfg(feature = "html")]
    // MarkdownElement::HTMLElement { source: _, element } => {
    //     fn emit_element(
    //         element: &lightml::Element<'_>,
    //         out: &mut impl Write,
    //         emitter: &impl FeatureEmitter,
    //         options: ParseOptions,
    //     ) -> Result<(), Box<dyn std::error::Error>> {
    //         write!(out, "<{tag_name}", tag_name = element.tag_name)?;
    //         for lightml::Attribute { key, value } in &element.attributes {
    //             write!(out, " \"{key}\"=\"{value}\"")?;
    //         }
    //         writeln!(out, ">")?;
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
    //                 writeln!(out, "</{tag_name}>", tag_name = element.tag_name)?;
    //             }
    //             lightml::ElementChildren::SelfClosing => {}
    //             lightml::ElementChildren::Literal(ref content) => {
    //                 writeln!(out, "{content}\n</{tag_name}>", tag_name = element.tag_name)?;
    //             }
    //         }
    //         Ok(())
    //     }

    //     let _ = emit_element(&element, out, emitter, options)?;
    // }

    Ok(())
}

pub fn inner_to_html(
    out: &mut impl Write,
    emitter: &impl FeatureEmitter,
    content: RawText,
) -> Result<(), Box<dyn std::error::Error>> {
    fn update_decoration(
        out: &mut impl Write,
        opening: TextDecoration,
        closing: TextDecoration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if opening.contains(TextDecoration::BOLD) {
            write!(out, "<bold>")?;
        } else if closing.contains(TextDecoration::BOLD) {
            write!(out, "</bold>")?;
        }
        if opening.contains(TextDecoration::EMPHASIS) {
            write!(out, "<em>")?;
        } else if closing.contains(TextDecoration::EMPHASIS) {
            write!(out, "</em>")?;
        }
        if opening.contains(TextDecoration::HIGHLIGHTED) {
            // FUTURE change default
            write!(out, "<span style=\"background-color: yellow\">")?;
        } else if closing.contains(TextDecoration::HIGHLIGHTED) {
            write!(out, "</span>")?;
        }
        if opening.contains(TextDecoration::STRIKETHROUGH) {
            write!(out, "<s>")?;
        } else if closing.contains(TextDecoration::STRIKETHROUGH) {
            write!(out, "</s>")?;
        }
        if opening.contains(TextDecoration::SUBSCRIPT) {
            write!(out, "<sub>")?;
        } else if closing.contains(TextDecoration::SUBSCRIPT) {
            write!(out, "</sub>")?;
        }
        if opening.contains(TextDecoration::SUPERSCRIPT) {
            write!(out, "<sup>")?;
        } else if closing.contains(TextDecoration::SUPERSCRIPT) {
            write!(out, "</sup>")?;
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
        update_decoration(out, difference & decoration, difference & current)?;
        match kind {
            MarkdownPart::Plain => write!(out, "{on}", on = escape_string_content(on))?,
            MarkdownPart::InlineCode => {
                write!(out, "<code>{on}</code>", on = escape_string_content(on))?;
            }
            MarkdownPart::InlineMathematics => write!(
                out,
                "<span class=\"mathematics inline\">{out}</span>",
                out = emitter.mathematics(on, false)
            )?,
            MarkdownPart::Emoji => write!(out, "")?,
            MarkdownPart::Tag => write!(out, "")?,
            MarkdownPart::Interpolation => write!(out, "")?,
            MarkdownPart::RawLink => write!(out, "<a href=\"{on}\">{on}</a>")?,
            MarkdownPart::ExternalLink { to } => write!(
                out,
                "<a href=\"{to}\">{on}</a>",
                on = escape_string_content(on)
            )?,
            MarkdownPart::MediaLink { source } => write!(
                out,
                "<img src=\"{source}\" alt=\"{on}\">",
                on = escape_string_content(on)
            )?,
            MarkdownPart::LineBreak => write!(out, "<br>",)?,
            // FUTURE improve
            MarkdownPart::InternalLink { to } => write!(
                out,
                "<a href=\"#{to}\">{on}</a>",
                on = escape_string_content(on)
            )?,
            MarkdownPart::HTMLElement(element) => write!(out, "{content}", content = element.0)?,
        }
        current = decoration;
    }
    update_decoration(out, TextDecoration::NONE, current)?;
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
