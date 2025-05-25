fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let content = if let Some(path) = args.pop_front() {
        let content = std::fs::read_to_string(path)?;
        content
    } else {
        //         "> [!INFO]
        // > Note that events are inferred. This isn't something you have to annotate.

        // > #TODO correlation between term types and function types. Giving it more information beyond just the type information. May actually be more confusing.".to_owned()
        // ---
        // layout: post.njk
        // title: Recording and replaying events
        // date: 2025-11-20
        // description: The side-effect experiment
        // image: /media/banners/todo.png
        // tags:
        // - posts
        // ---
        "Subtyping is the asking of a question whether one type is contained in another two. We can extend our Venn diagrams with an extra dimension and consider by considering one type being overlaid by another type. *If I have it all working correctly*, you should be able to drag the sliders to move the sets each other. For the overlaid set to be a subtype we want to see no red colour here at the end of the end of the translation. We see that sets on the right end with items that don't fit into the base type and therefore is not a subtype. If we have no red, then we have that the top type is a subtype.

{% includewidget 'three-dimensional-set.html' /%}

> Note that this is a relation not an equality. At the end of the transition we can have left over yellow.
"
        .to_owned()
    };

    fn handler(item: simple_markdown_parser::MarkdownElement, indent: usize) -> Result<(), ()> {
        if let simple_markdown_parser::MarkdownElement::CommandBlock(block) = item {
            eprint!(
                "MarkdownElement::CommandBlock {{ name: {name:?}, arguments: {arguments:?} }}",
                name = block.name,
                arguments = block.arguments()
            );
            if !block.inner.0.is_empty() {
                let options = simple_markdown_parser::ParseOptions::default();
                eprintln!(" [");
                let _ =
                    simple_markdown_parser::parse_with_options(block.inner.0, options, 0, |item| {
                        handler(item, indent + 1)
                    });
                eprint!("] End of {name:?}", name = block.name);
            }
            eprintln!();
        } else {
            for _ in 0..indent {
                eprint!("\t");
            }
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

        Ok(())
    }

    let options = simple_markdown_parser::ParseOptions::default();
    let result = simple_markdown_parser::parse_with_options(content.as_str(), options, 0, |item| {
        handler(item, 0)
    });

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
