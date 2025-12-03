fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<_> = std::env::args().skip(1).rev().collect();
    let path = args.pop().unwrap_or("./private/corpus/aaa.md".into());
    let content = std::fs::read_to_string(path)?;

    fn handler(item: simple_markdown_parser::MarkdownElement) -> Result<(), ()> {
        println!("{}", item.debug_with_options(Default::default()));
        Ok(())
    }

    let mut options = simple_markdown_parser::ParseOptions::default();
    options.heading_underscores = true;
    let result = simple_markdown_parser::parse_with_options(
        content.as_str(),
        options,
        Default::default(),
        handler,
    );

    match result {
        Ok(()) => {
            eprintln!("finished :)");
        }
        Err(err) => {
            eprintln!("error {err:?}");
        }
    }
    Ok(())
}
