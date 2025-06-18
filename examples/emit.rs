fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let content = if let Some(path) = args.pop_front() {
        std::fs::read_to_string(path)?
    } else {
        "# Hello world".to_string()
    };

    let out = args
        .pop_front()
        .map_or("./private/html/demo.html".into(), std::path::PathBuf::from);

    let mut file = std::fs::File::create(out).unwrap();

    let _ = simple_markdown_parser::emit::markdown_to_html(
        &content,
        &mut file,
        &mut simple_markdown_parser::emit::BlankFeatureEmitter,
        simple_markdown_parser::ParseOptions::default(),
        Default::default(),
    );

    Ok(())
}
