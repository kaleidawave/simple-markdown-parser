use simple_markdown_parser::{parse, MarkdownElement};

fn as_lines(content: &str) -> String {
    use std::fmt::Write;

    let mut buf = String::new();
    // TODO specify by flags
    let options = simple_markdown_parser::ParseOptions {
        allow_asterisk_and_plus_as_list_prefixes: true,
        heading_underscores: true,
        indented_code_blocks: true,
        tilda_code_blocks: true,
        record_empty_lines: false,
    };
    let () = simple_markdown_parser::parse_with_options::<()>(content, options, 0, |item| {
        writeln!(buf, "{item}", item = item.debug_with_options(true)).unwrap();
        Ok(())
    })
    .unwrap();
    buf
}

fn main() -> std::process::ExitCode {
    let tests = get_tests();

    println!();
    let count = tests.len();
    println!("running {count} tests");

    let mut failures: Vec<String> = Default::default();

    let now = std::time::Instant::now();

    for test_case in tests {
        fn test<F>(_name: &str, cb: F) -> Result<(), ()>
        where
            F: FnOnce() + std::marker::Send + 'static,
        {
            let res = std::thread::spawn(cb);
            match res.join() {
                Ok(()) => Ok(()),
                Err(_) => Err(()),
            }
        }

        // let name = format!("{name}");
        let name = test_case.name;
        let result = test(&name, move || {
            let out = as_lines(&test_case.case).replace("\r\n", "\n");
            let expectation = test_case.output.trim_end();
            pretty_assertions::assert_eq!(out.trim_end(), expectation, "expected {expectation}",);
        });
        if let Ok(()) = result {
            println!("test {name} ... \u{001b}\u{005b}\u{0033}\u{0032}\u{006d}\u{006f}\u{006b}\u{001b}\u{005b}\u{0033}\u{0039}\u{006d}");
        } else {
            println!("test {name} ... \u{001b}\u{005b}\u{0033}\u{0031}\u{006d}\u{0066}\u{0061}\u{0069}\u{006c}\u{0065}\u{0064}\u{001b}\u{005b}\u{0033}\u{0039}\u{006d}");
            failures.push(name.to_string());
        }
    }

    {
        let elapsed = now.elapsed();
        let result = if failures.is_empty() { "ok" } else { "err" };
        let passed = count - failures.len();
        let failed = failures.len();
        // FUTURE will we support this?
        let ignored = 0;
        let measured = 0;
        let filtered_out = 0;
        eprintln!("\ntest result: {result}. {passed} passed; {failed} failed; {ignored} ignored; {measured} measured; {filtered_out} filtered out; finished in {elapsed:?}");
    }

    if failures.is_empty() {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

#[derive(Debug, Default)]
struct Test {
    section: String,
    name: String,
    options: (),
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
        } else if let MarkdownElement::CodeBlock(simple_markdown_parser::CodeBlock {
            code, ..
        }) = element
        {
            let code = code.replace("\r\n", "\n");
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
