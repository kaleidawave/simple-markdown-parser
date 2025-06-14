fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: Vec<_> = std::env::args().skip(1).rev().collect();
    let content = if let Some(path) = args.pop() {
        std::fs::read_to_string(path)?
    } else {
        todo!()
    };

    let options = Default::default();

    #[derive(Debug, Default)]
    struct Statistics {
        pub h1: usize,
        pub h2: usize,
        pub h3: usize,
        pub h4: usize,
        pub h5: usize,
        pub h6: usize,
        pub paragraph: usize,
        pub codeblocks: usize,
        pub languages: std::collections::HashMap<String, usize>,
        pub quote: usize,
        pub list_items: usize,
    }

    let mut statistics = Statistics::default();

    simple_markdown_parser::parse_with_options::<()>(content.as_str(), options, 0, |item| {
        use simple_markdown_parser::MarkdownElement;
        match item {
            MarkdownElement::Heading {
                level: 1,
                content: _,
            } => statistics.h1 += 1,
            MarkdownElement::Heading {
                level: 2,
                content: _,
            } => statistics.h2 += 1,
            MarkdownElement::Heading {
                level: 3,
                content: _,
            } => statistics.h3 += 1,
            MarkdownElement::Heading {
                level: 4,
                content: _,
            } => statistics.h4 += 1,
            MarkdownElement::Heading {
                level: 5,
                content: _,
            } => statistics.h5 += 1,
            MarkdownElement::Heading {
                level: 6,
                content: _,
            } => statistics.h6 += 1,
            MarkdownElement::Paragraph(_) => statistics.paragraph += 1,
            MarkdownElement::CodeBlock(cb) => {
                statistics.codeblocks += 1;
                *statistics
                    .languages
                    .entry(cb.language.to_owned())
                    .or_default() += 1;
            }
            MarkdownElement::Quote(_) => statistics.quote += 1,
            MarkdownElement::ListItem { .. } => statistics.list_items += 1,
            MarkdownElement::CommentBlock(_) => {}
            item => eprintln!("Not recording {item:?}"),
        }
        Ok(())
    })
    .unwrap();

    println!("{statistics:#?}");

    Ok(())
}
