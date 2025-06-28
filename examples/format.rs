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
    // #[derive(PartialEq, Eq, Clone, Copy)]
    // enum ItemKind {
    //     Start,
    //     Media,
    //     Other,
    // }

    let first_new_line = content.find('\n').unwrap_or_default();
    let uses_crlf = content[..first_new_line].ends_with('\r');
    let line_end = if uses_crlf { "\r\n" } else { "\n" };

    let parse_options = Default::default();
    let to_md_options = simple_markdown_parser::extras::AsMarkdownOptions {
        uses_crlf,
        skip_comments: false,
    };
    let mut buf: Vec<u8> = Vec::new();
    // let mut last_was_block = ItemKind::Start;

    simple_markdown_parser::parse_with_options::<()>(
        content,
        parse_options,
        Default::default(),
        |item| {
            use std::io::Write;

            if !buf.is_empty() {
                write!(&mut buf, "{line_end}").unwrap();
            }
            let _ = &item.as_markdown(&mut buf, "", to_md_options);
            write!(&mut buf, "{line_end}").unwrap();
            Ok(())
        },
    )
    .unwrap();

    unsafe { String::from_utf8_unchecked(buf) }
}
