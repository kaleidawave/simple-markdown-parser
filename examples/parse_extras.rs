fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let path = args.pop_front().ok_or("expected argument")?;
    let content = std::fs::read_to_string(path)?;

    let _ = simple_markdown_parser::utilities::parse_blocks(&content, |headings, items| {
        eprint!("{headings:?} -> [\n");
        for item in items {
            eprintln!("\t{item:?},");
        }
        eprint!("]\n\n");
    });

    eprintln!("finished");

    Ok(())
}
