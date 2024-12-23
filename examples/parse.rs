fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let path = args.pop_front().ok_or("expected argument")?;
    let content = std::fs::read_to_string(path)?;

    fn handler(item: simple_markdown_parser::MarkdownElement) {
        if let simple_markdown_parser::MarkdownElement::CommandBlock(block) = item {
            eprintln!(
                "MarkdownElement::CommandBlock {{ name: {name:?}, arguments: {arguments:?} }} [",
                name = block.name,
                arguments = block.arguments()
            );
            let _ = simple_markdown_parser::parse(&block.inner.0, handler);
            eprintln!("] End of {name:?}", name = block.name);
            return;
        } else {
            if let Some(parts) = item.parts_like() {
                eprint!("{} -> ", item.debug_without_text());
                eprintln!(
                    "parts={inner:?}",
                    inner = parts
                        .parts()
                        .flat_map(|part| match part {
                            simple_markdown_parser::MarkdownTextElement::Link { on, .. } => {
                                on.parts().collect::<Vec<_>>()
                            }
                            part => vec![part],
                        })
                        .collect::<Vec<_>>()
                );
            } else {
                eprintln!("{:?}", item);
            }
        }
    }

    let _ = simple_markdown_parser::parse(&content, handler);

    eprintln!("finished");

    Ok(())
}
