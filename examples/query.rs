fn main() -> Result<(), Box<dyn std::error::Error>> {
    use simple_markdown_parser::utilities::extraction::{between_headers, links, Stop};

    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let path = args.pop_front().unwrap();
    let content = std::fs::read_to_string(path.clone())?;

    let arg = args.pop_front();

    if let Some(arg) = arg.as_deref() {
        match arg {
            "links" => {
                links(&content, |name, link| eprintln!("{name} -> {link}"));
            }
            "heading" => {
                let start = args.pop_front();
                let arg = args.pop_front();
                let stop = if let Some(stop) = arg.as_deref() {
                    Some(match stop {
                        "fs" => Stop::FirstSection,
                        "ml" => Stop::MatchingLevel,
                        stop => {
                            if let Some(heading) = stop.strip_prefix("heading ") {
                                Stop::AtHeader(heading)
                            } else {
                                return Err(Box::<dyn std::error::Error>::from(format!(
                                    "Do not know how to interpret {stop}"
                                )));
                            }
                        }
                    })
                } else {
                    None
                };

                let result = between_headers(&content, start.as_deref(), stop).trim();
                eprintln!("{result}");
            }
            arg => {
                panic!("Unknown option {arg}")
            }
        }
    }
    Ok(())
}
