use crate::{MarkdownElement, MarkdownTextElement, RawText};
use std::io::Write;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
pub fn markdown_to_html_string(source: &str, emitter: Option<FeatureEmitterWASM>) -> String {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    let mut bytes: Vec<u8> = Vec::new();
    let _ = match emitter {
        Some(mut emitter) => markdown_to_html(source, &mut bytes, &mut emitter),
        None => markdown_to_html(source, &mut bytes, &mut BlankFeatureEmitter),
    };
    match String::from_utf8(bytes) {
        Ok(result) => result,
        Err(_) => String::from("Non Utf8 output or markdown parser error"),
    }
}

pub fn markdown_to_html(
    source: &str,
    out: &mut impl Write,
    emitter: &mut impl FeatureEmitter,
) -> Result<(), ()> {
    let mut last_was_list_item: bool = false;
    crate::parse(source, |item| {
        let is_list_item = matches!(&item, MarkdownElement::ListItem { .. });
        if !last_was_list_item && is_list_item {
            writeln!(out, "<ul>").unwrap();
        } else if last_was_list_item && !is_list_item {
            writeln!(out, "</ul>").unwrap();
        }
        element_to_html(out, emitter, item).unwrap();
        last_was_list_item = is_list_item;
    })
}

pub trait FeatureEmitter {
    fn code_block(&self, language: &str, code: &str) -> String;

    fn latex(&self, code: &str) -> String;

    fn command(&self, name: &str, args: Vec<(&str, &str)>, inner: &str) -> String;

    fn interpolation(&self, expression: &str) -> String;
}

/// Un-highlighted code and panics on `RegExp`
pub struct BlankFeatureEmitter;

impl FeatureEmitter for BlankFeatureEmitter {
    fn code_block(&self, _language: &str, code: &str) -> String {
        code.to_owned()
    }

    fn latex(&self, _code: &str) -> String {
        panic!("`BlankFeatureEmitter` does implement LaTeX HTML generation");
    }

    fn command(&self, _name: &str, _args: Vec<(&str, &str)>, _inner: &str) -> String {
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
    latex_callback: js_sys::Function,
    command_callback: js_sys::Function,
    interpolation_callback: js_sys::Function,
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
export class FeatureEmitterWASM { 
    constructor(
        code_block_callback: (language: string, code: string) => string,
        latex_callback: (code: string) => string,
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
        latex_callback: js_sys::Function,
        command_callback: js_sys::Function,
        interpolation_callback: js_sys::Function,
    ) -> Self {
        Self {
            code_block_callback,
            latex_callback,
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

    fn latex(&self, code: &str) -> String {
        let result = self
            .latex_callback
            .call1(&JsValue::NULL, &JsValue::from_str(code));
        result_to_string(result)
    }

    fn command(&self, name: &str, args: Vec<(&str, &str)>, inner: &str) -> String {
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
        result_to_string(result)
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
    emitter: &mut impl FeatureEmitter,
    item: MarkdownElement,
) -> Result<(), Box<dyn std::error::Error>> {
    match item {
        MarkdownElement::Heading { level, text } => {
            assert!(level < 7, "heading level too much for HTML");
            writeln!(out, "<h{level}>")?;
            inner_to_html(out, emitter, text)?;
            writeln!(out, "</h{level}>")?;
        }
        MarkdownElement::Quote(_text) => {
            writeln!(out, "<blockquote>")?;
            // TODO
            // inner_to_html(out, emitter, text)?;
            writeln!(out, "</blockquote>")?;
        }
        MarkdownElement::Paragraph(text) => {
            if text.0.starts_with("![") || text.0.starts_with("[![") {
                // Don't wrap media in `<p>`
                inner_to_html(out, emitter, text)?;
            } else {
                writeln!(out, "<p>")?;
                inner_to_html(out, emitter, text)?;
                writeln!(out, "</p>")?;
            }
        }
        MarkdownElement::ListItem {
            level: _level,
            text,
        } => {
            writeln!(out, "<li>")?;
            inner_to_html(out, emitter, text)?;
            writeln!(out, "</li>")?;
        }
        // TODO
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
        MarkdownElement::CodeBlock { language, code } => {
            let inner = emitter.code_block(language, code);
            writeln!(out, "<pre>{inner}</pre>")?;
        }
        MarkdownElement::LaTeXBlock { script: _ } => {}
        // TODO how much to do here
        MarkdownElement::HTMLElement(_) => {}
        // TODO at start?
        MarkdownElement::Frontmatter(inner) => {
            writeln!(out, "<pre>{inner}</pre>")?;
        }
        MarkdownElement::HorizontalRule => {
            writeln!(out, "<hr>")?;
        }
        MarkdownElement::CommandBlock(command) => {
            writeln!(
                out,
                "{result}",
                result = emitter.command(command.name, command.arguments(), command.inner.0)
            )?;
        }
        // MarkdownElement::Media {
        //     alt: _,
        //     link: _,
        //     source: _,
        // } => {}
        MarkdownElement::Footnote => {}
        MarkdownElement::CommentBlock(_) | MarkdownElement::Empty => {}
    }

    Ok(())
}

pub fn inner_to_html(
    out: &mut impl Write,
    emitter: &mut impl FeatureEmitter,
    text: RawText,
) -> Result<(), Box<dyn std::error::Error>> {
    for part in text.parts() {
        text_element_to_html(out, emitter, part)?;
    }
    Ok(())
}

#[allow(clippy::match_same_arms)]
pub fn text_element_to_html(
    out: &mut impl Write,
    emitter: &mut impl FeatureEmitter,
    item: MarkdownTextElement,
) -> Result<(), Box<dyn std::error::Error>> {
    match item {
        MarkdownTextElement::Plain(content) => write!(out, "{content}")?,
        MarkdownTextElement::Bold(content) => write!(out, "<strong>{content}</strong>")?,
        MarkdownTextElement::Italic(content) => write!(out, "<em>{content}</em>")?,
        MarkdownTextElement::BoldAndItalic(content) => {
            write!(out, "<strong><em>{content}</em></strong>")?;
        }
        MarkdownTextElement::Code(content) => write!(out, "<code>{content}</code>")?,
        MarkdownTextElement::StrikeThrough(content) => write!(out, "{content}")?,
        MarkdownTextElement::Emoji(content) => write!(out, "{content}")?,
        MarkdownTextElement::Latex(content) => write!(out, "{content}")?,
        MarkdownTextElement::Highlight(content) => write!(out, "{content}")?,
        MarkdownTextElement::Subscript(content) => write!(out, "{content}")?,
        MarkdownTextElement::Superscript(content) => write!(out, "{content}")?,
        MarkdownTextElement::Tag(content) => write!(out, "{content}")?,
        MarkdownTextElement::Media { alt, source } => {
            // TODO videos?
            write!(out, "<img alt=\"{alt}\" src=\"{source}\">")?;
        }
        MarkdownTextElement::Expression(item) => {
            write!(out, "{result}", result = emitter.interpolation(item))?;
        }
        MarkdownTextElement::Link { on, to } => {
            write!(out, "<a href=\"{to}\">")?;
            inner_to_html(out, emitter, on)?;
            write!(out, "</a>")?;
        }
    };

    Ok(())
}
