use simple_markdown_parser::{MarkdownElement, RawText};

#[test]
fn scan() {
    let source: &str = r"
# Hello world

## Under heading

```ts
this is some code
```

- List item 1
- List item 2

> Some block quote

---

## Another item

Paragraph here
";

    let mut output = Vec::new();

    let _ = simple_markdown_parser::utilities::parse_with_header_information(
        source,
        |header_chain, item| {
            output.push((header_chain.to_owned(), item));
        },
    );

    let expected = vec![
        (
            vec![],
            MarkdownElement::Heading {
                level: 1,
                text: RawText("Hello world"),
            },
        ),
        (
            vec![RawText("Hello world")],
            MarkdownElement::Heading {
                level: 2,
                text: RawText("Under heading"),
            },
        ),
        (
            vec![RawText("Hello world"), RawText("Under heading")],
            MarkdownElement::CodeBlock {
                language: "ts",
                content: "this is some code\n",
            },
        ),
        (
            vec![RawText("Hello world"), RawText("Under heading")],
            MarkdownElement::ListItem {
                level: 1,
                text: RawText("List item 1"),
            },
        ),
        (
            vec![RawText("Hello world"), RawText("Under heading")],
            MarkdownElement::ListItem {
                level: 1,
                text: RawText("List item 2"),
            },
        ),
        (
            vec![RawText("Hello world"), RawText("Under heading")],
            MarkdownElement::Quote(RawText(" Some block quote")),
        ),
        (
            vec![RawText("Hello world"), RawText("Under heading")],
            MarkdownElement::HorizontalRule,
        ),
        (
            vec![RawText("Hello world")],
            MarkdownElement::Heading {
                level: 2,
                text: RawText("Another item"),
            },
        ),
        (
            vec![RawText("Hello world"), RawText("Another item")],
            MarkdownElement::Paragraph(RawText("Paragraph here")),
        ),
    ];

    pretty_assertions::assert_eq!(output, expected);
}
