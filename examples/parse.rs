fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let path = args.pop_front().ok_or("expected argument")?;
    let content = std::fs::read_to_string(path)?;

    let _ = simple_markdown_parser::parse(&content, |item| {
        eprintln!("{item:?}");

        if let simple_markdown_parser::MarkdownElement::Paragraph(item) = item {
            eprintln!(
                "parts={inner:?}",
                inner = item
                    .parts()
                    .flat_map(|part| match part {
                        simple_markdown_parser::MarkdownTextElement::Link { on, .. } => {
                            let mut inner = vec![part];
                            inner.extend(on.parts());
                            inner
                        }
                        part => vec![part],
                    })
                    .collect::<Vec<_>>()
            );
        }
    });

    eprintln!("finished");

    Ok(())
}
