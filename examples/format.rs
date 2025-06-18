fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<_> = std::env::args().skip(1).rev().collect();
    let root = args.pop().expect("no entry point");

    let mut dry_run = false;
    let mut check = false;
    match args.pop().as_deref() {
        Some("--dry-run") => {
            dry_run = true;
        }
        Some("--check") => {
            check = true;
        }
        Some(flag) => {
            eprintln!("Unknown flag '{flag}'");
            return Ok(());
        }
        None => {}
    }

    for entry in glob::glob(&root).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                let content = std::fs::read_to_string(&path).expect("Could not read file");
                let formatted = format_markdown_content(&content);
                if dry_run {
                    println!("{formatted}");
                } else if check {
                    pretty_assertions::assert_eq!(formatted, content);
                    eprintln!("Matched");
                } else if content != formatted {
                    eprintln!("Modified {path}", path = path.display());
                    std::fs::write(path, formatted).expect("failed to write to file");
                }
            }
            Err(e) => eprintln!("path error: {e:?}"),
        }
    }

    Ok(())
}

fn format_markdown_content(content: &str) -> String {
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum ItemKind {
        Start,
        Media,
        ListItem,
        Other,
    }

    let first_new_line = content.find('\n').unwrap_or_default();
    let uses_crlf = content[..first_new_line]
        .chars()
        .next_back()
        .is_some_and(|l| l == '\r');
    let line_end = if uses_crlf { "\r\n" } else { "\n" };

    let parse_options = Default::default();
    let to_string_options = simple_markdown_parser::utilities::AsMarkdownOptions {
        uses_crlf,
        skip_comments: false,
    };
    let mut buf = String::new();
    let mut last_was_block = ItemKind::Start;

    simple_markdown_parser::parse_with_options::<()>(
        content,
        parse_options,
        Default::default(),
        |item| {
            use std::fmt::Write;

            let item_is_block = match item {
                // simple_markdown_parser::MarkdownElement::ListItem { .. } => ItemKind::ListItem,
                simple_markdown_parser::MarkdownElement::Paragraph(item) => {
                    // WIP
                    if item.0.starts_with("![") || item.0.starts_with("[![") {
                        ItemKind::Media
                    } else {
                        ItemKind::Other
                    }
                }
                _ => ItemKind::Other,
            };
            let should_add_line = match (last_was_block, item_is_block) {
                (ItemKind::Start, _) => false,
                (ItemKind::ListItem, ItemKind::ListItem) => false,
                (ItemKind::Media, ItemKind::Media) => false,
                _ => true,
            };
            if should_add_line {
                write!(&mut buf, "{line_end}").unwrap();
            }
            buf.push_str(&item.as_markdown(to_string_options));
            last_was_block = item_is_block;
            write!(&mut buf, "{line_end}").unwrap();
            Ok(())
        },
    )
    .unwrap();

    buf
}
