fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let content = if let Some(path) = args.pop_front() {
        let content = std::fs::read_to_string(path)?;
        content
    } else {
        "# Hello world".to_string()
    };

    let out = args
        .pop_front()
        .map(std::path::PathBuf::from)
        .unwrap_or("./private/html/demo.html".into());

    let mut file = std::fs::File::create(out).unwrap();

    let _ = simple_markdown_parser::extras::emit::markdown_to_html(
        &content,
        &mut file,
        &mut simple_markdown_parser::extras::emit::BlankFeatureEmitter,
        simple_markdown_parser::ParseOptions::default(),
        0,
    );

    Ok(())
}
