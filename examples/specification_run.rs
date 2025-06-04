use simple_markdown_parser::{parse, CodeBlock, MarkdownElement};

fn as_lines(content: &str) -> String {
    use std::fmt::Write;

    let mut buf = String::new();
    // TODO specify by flags
    let options = simple_markdown_parser::ParseOptions {
        allow_asterisk_and_plus_as_list_prefixes: true,
        heading_underscores: true,
    };
    let _result = simple_markdown_parser::parse_with_options::<()>(content, options, 0, |item| {
        writeln!(buf, "{item}", item = item.debug_with_options(true)).unwrap();
        Ok(())
    });
    buf
}

// TODO glob syntax
struct Pattern<'a>(pub Vec<&'a str>);

fn main() {
    let tests = get_tests();

    let mut args = std::env::args().skip(1);

    let pattern;

    // TODO temp
    let include = if let Some(arg) = args.next() {
        if let "--only" = arg.as_str() {
            pattern = args.next().unwrap();
            Some(Pattern(pattern.split(',').collect()))
        } else {
            None
        }
    } else {
        None
    };

    println!();
    println!("running {count} tests", count = tests.len());

    // let mut failures: Vec<String> = Default::default();

    for test_case in tests.into_iter() {
        // fn test<F>(_name: &str, cb: F) -> Result<(), ()>
        // where
        //     F: FnOnce() -> () + std::marker::Send + 'static,
        // {
        //     let res = std::thread::spawn(cb);
        //     match res.join() {
        //         Ok(_) => Ok(()),
        //         Err(_) => Err(()),
        //     }
        // }

        // let name = format!("{name}");
        let name = test_case.name;
        let section = test_case.section;

        let whole_name = format!("{section} / {name}",).to_lowercase();

        let run = if let Some(ref pattern) = include {
            pattern.0.iter().any(|case| whole_name.contains(case))
        } else {
            true
        };

        if run {
            // let result = test(&name, move || {
            let out = as_lines(&test_case.case).replace("\n", "\r\n");

            if out == test_case.output {
                continue;
            }

            eprintln!(
                "input '{name}':\n{case}\nrecieved:\n{out}\n---",
                case = test_case.case
            );

            let pause = true;
            if pause {
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read line");
                println!("\x1b[2J\x1b[H");

                if let "exit" | "e" | "quit" | "q" = input.as_str().trim() {
                    break;
                }
            }
        }

        // let expectation = test_case.output.trim_end();
        // assert_eq!(out.trim_end(), expectation, "expected {out}",)
        // });
        // match result {
        //     Ok(()) => {
        //         println!("test {name} ... \u{001b}\u{005b}\u{0033}\u{0032}\u{006d}\u{006f}\u{006b}\u{001b}\u{005b}\u{0033}\u{0039}\u{006d}");
        //     }
        //     Err(()) => {
        //         println!("test {name} ... \u{001b}\u{005b}\u{0033}\u{0031}\u{006d}\u{0066}\u{0061}\u{0069}\u{006c}\u{0065}\u{0064}\u{001b}\u{005b}\u{0033}\u{0039}\u{006d}");
        //         failures.push(name.to_string());
        //     }
        // }
    }

    // TODO failures
}

#[derive(Debug, Default)]
struct Test {
    section: String,
    name: String,
    _options: (),
    case: String,
    output: String,
}

fn get_tests() -> Vec<Test> {
    let mut tests: Vec<Test> = Vec::new();
    let mut current_test = Test::default();
    let mut section = String::new();
    let result = parse::<()>(include_str!("../tests/specification.md"), |element| {
        if let MarkdownElement::Heading { level, content } = element {
            if level >= 3 {
                if !current_test.case.is_empty() {
                    tests.push(std::mem::take(&mut current_test));
                }
                current_test.name = content.no_decoration();
                section.clone_into(&mut current_test.section);
            } else {
                section = content.no_decoration();
            }
        } else if let MarkdownElement::Paragraph(_content) = element {
            // if content.0.ends_with("`top_level_separator = Some(\"\\n\")`") {
            //     current_test.options.top_level_separator = Some("\n");
            // }
        } else if let MarkdownElement::CodeBlock(CodeBlock { code, .. }) = element {
            if current_test.case.is_empty() {
                code.clone_into(&mut current_test.case);
            } else if current_test.output.is_empty() {
                code.clone_into(&mut current_test.output);
            } else {
                let next_name = format!("{} *", current_test.name);
                tests.push(std::mem::take(&mut current_test));
                current_test.name = next_name;
            }
        }
        Ok(())
    });

    assert!(result.is_ok(), "{result:?}");
    if !current_test.case.is_empty() {
        tests.push(current_test);
    }
    tests
}
