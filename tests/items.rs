#[test]
fn scan() {
    let source: &str = r#"
# The `parse` API

The `parse` API takes a callback that is called on each [markdown](https://www.markdownguide.org/) item. The inner text decoration is **skipped** on the first pass but can be *picked up* later using `RawText::parts`, which returns a `PartsIterator` to get the individual sections.
"#;

    let _ = simple_markdown_parser::parse(source, |item| {
        use simple_markdown_parser::{MarkdownElement, MarkdownTextElement, RawText};
        match item {
            MarkdownElement::Heading { level: 1, text } => {
                pretty_assertions::assert_eq!(
                    text.parts().collect::<Vec<_>>(),
                    vec![
                        MarkdownTextElement::Plain("The "),
                        MarkdownTextElement::Code("parse"),
                        MarkdownTextElement::Plain(" API"),
                    ]
                );
            }
            MarkdownElement::Paragraph(text) => {
                pretty_assertions::assert_eq!(
                    text.parts().collect::<Vec<_>>(),
                    vec![
                        MarkdownTextElement::Plain("The "),
                        MarkdownTextElement::Code("parse"),
                        MarkdownTextElement::Plain(" API takes a callback that is called on each "),
                        MarkdownTextElement::Link {
                            on: RawText("markdown"),
                            to: "https://www.markdownguide.org/"
                        },
                        MarkdownTextElement::Plain(" item. The inner text decoration is "),
                        MarkdownTextElement::Bold("skipped"),
                        MarkdownTextElement::Plain(" on the first pass but can be "),
                        MarkdownTextElement::Italic("picked up"),
                        MarkdownTextElement::Plain(" later using "),
                        MarkdownTextElement::Code("RawText::parts"),
                        MarkdownTextElement::Plain(", which returns a "),
                        MarkdownTextElement::Code("PartsIterator"),
                        MarkdownTextElement::Plain(" to get the individual sections.")
                    ]
                );
            }
            item => panic!("not expecting {item:?}"),
        }
    });
}
