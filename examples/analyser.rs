fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args: std::collections::VecDeque<_> = std::env::args().skip(1).collect();
    let path = args.pop_front().ok_or("expected argument")?;
    let content = std::fs::read_to_string(path)?;

    #[derive(Debug, Default)]
    struct LexicalAnalyser1 {
        words: std::collections::HashMap<String, usize>,
        sentences: Vec<String>,
        paragraphs: Vec<String>,
    }

    impl simple_markdown_parser::utilities::LexicalAnalyser for LexicalAnalyser1 {
        fn word(&mut self, word: &str) {
            self.words
                .entry(word.trim().to_lowercase())
                .and_modify(|counter| *counter += 1)
                .or_insert(1);
        }

        /// **WARNING** called with decoration
        fn sentence(&mut self, sentence: &str) {
            self.sentences.push(sentence.to_owned());
        }

        /// **WARNING** called with decoration
        fn paragraph(&mut self, paragraph: &str) {
            self.paragraphs.push(paragraph.to_owned());
        }
    }

    let mut analyser = LexicalAnalyser1::default();

    let _ = simple_markdown_parser::utilities::lexical_analysis(&content, &mut analyser);

    eprintln!("Finished:");
    eprintln!("\tWords: {words:#?}", words = analyser.words);
    eprintln!(
        "\tsentences: {sentences}",
        sentences = analyser.sentences.len()
    );
    eprintln!(
        "\tParagraphs: {paragraphs}",
        paragraphs = analyser.paragraphs.len()
    );

    Ok(())
}
