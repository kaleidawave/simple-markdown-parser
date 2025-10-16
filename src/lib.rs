#![doc = include_str!("../README.md")]

pub mod emit;
pub mod extras;
pub mod parser;
pub mod utilities;

pub use parser::{ContainerResidue, MarkdownParser, ParseOptions, PartsIterator, TableRow};

/// Markdown block element
#[derive(Debug, Copy, Clone)]
pub enum MarkdownElement<'a> {
    Heading {
        level: u8,
        content: RawText<'a>,
    },
    Quote(QuoteBlock<'a>),
    Paragraph(RawText<'a>),
    List(List<'a>),
    Table(Table<'a>),
    // TODO modifiers
    CodeBlock(CodeBlock<'a>),
    MathematicsBlock(MathematicsBlock<'a>),
    CommandBlock(CommandBlock<'a>),
    /// Inside `%%` (from Obsidan)
    CommentBlock(&'a str),
    /// Includes HTML comments
    HTMLElement(HTMLElement<'a>),
    // TODO at start?
    Frontmatter(Frontmatter<'a>),
    HorizontalRule,
    // Footnote,
    Empty,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum MarkdownPart<'a> {
    #[default]
    Plain,
    InlineCode,
    InlineMathematics,
    Emoji,
    Tag,
    Interpolation,
    // Links
    /// `<...>`
    RawLink,
    InternalLink {
        to: &'a str,
    },
    ExternalLink {
        to: &'a str,
    },
    MediaLink {
        source: &'a str,
        alt: &'a str,
    },
    LineBreak,
    HTMLElement(HTMLElement<'a>),
}

#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct TextDecoration(pub(crate) u8);

impl std::ops::BitXor for TextDecoration {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for TextDecoration {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl std::ops::BitAnd for TextDecoration {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for TextDecoration {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl TextDecoration {
    pub const NONE: TextDecoration = TextDecoration(0);

    // Basic decoration
    pub const BOLD: TextDecoration = TextDecoration(1 << 0);
    pub const EMPHASIS: TextDecoration = TextDecoration(1 << 1);

    // More
    pub const HIGHLIGHTED: TextDecoration = TextDecoration(1 << 2);
    pub const STRIKETHROUGH: TextDecoration = TextDecoration(1 << 3);

    // Above or below
    pub const SUPERSCRIPT: TextDecoration = TextDecoration(1 << 4);
    pub const SUBSCRIPT: TextDecoration = TextDecoration(1 << 5);

    #[must_use]
    pub fn contains(self, other: TextDecoration) -> bool {
        (self.0 & other.0) != 0
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MarkdownTextElement<'a> {
    pub on: &'a str,
    pub decoration: TextDecoration,
    pub kind: MarkdownPart<'a>,
}

impl<'a> MarkdownTextElement<'a> {
    pub fn is_plain(&self) -> bool {
        self.decoration.is_empty() && matches!(self.kind, MarkdownPart::Plain)
    }
}

/// (unsplit) Text inside markdown item
#[derive(Debug, Copy, Clone)]
pub struct RawText<'a>(pub &'a str, pub ContainerResidue<'a>);

impl<'a> RawText<'a> {
    #[must_use]
    pub fn parts(&self) -> PartsIterator<'a> {
        PartsIterator::new_with_container_residue(self.0, self.1)
    }
}

#[derive(Debug)]
pub enum MarkdownParseError<T> {
    FromCallback(T),
}

impl<T: std::fmt::Debug> std::fmt::Display for MarkdownParseError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        std::fmt::Debug::fmt(self, f)
    }
}

impl<T: std::fmt::Debug + std::fmt::Display> std::error::Error for MarkdownParseError<T> {}

/// # Errors
/// errors for unclosed blocks
pub fn parse<'a, T>(
    on: &'a str,
    cb: impl FnMut(MarkdownElement<'a>) -> Result<(), T>,
) -> Result<(), MarkdownParseError<T>> {
    parse_with_options(on, ParseOptions::default(), ContainerResidue::default(), cb)
}

pub fn parse_with_options<'a, T>(
    on: &'a str,
    options: ParseOptions,
    container_residue: ContainerResidue<'a>,
    cb: impl FnMut(MarkdownElement<'a>) -> Result<(), T>,
) -> Result<(), MarkdownParseError<T>> {
    MarkdownParser::parse_with_options(on, options, container_residue, cb)
}

#[derive(Debug, Copy, Clone)]
pub struct RawMarkdown<'a>(pub &'a str, pub ContainerResidue<'a>);

#[derive(Debug, Copy, Clone)]
pub struct Table<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct CommandBlock<'a> {
    pub name: &'a str,
    pub arguments: &'a str,
    pub inner: RawMarkdown<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct Frontmatter<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct QuoteBlock<'a> {
    /// [See GitHub markdown alerts](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts).
    /// Note this allows any alerts. It does not check from a defined list
    pub alert: Option<&'a str>,
    pub inner: RawMarkdown<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct CodeBlock<'a> {
    pub indented_block: bool,
    pub language: &'a str,
    // This can have artifacts of [`ContainerResidue`]
    pub raw_code: &'a str,
    pub container_residue: ContainerResidue<'a>,
}

#[derive(Debug, Copy, Clone)]
pub struct HTMLElement<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct MathematicsBlock<'a>(pub &'a str);

#[derive(Debug, Copy, Clone)]
pub struct List<'a>(pub RawMarkdown<'a>);

#[derive(Debug, Copy, Clone)]
pub struct ListItem<'a> {
    pub content: RawMarkdown<'a>,
    /// TODO probably need more options here
    pub enumerated: bool,
    /// from `- [x]` etc
    pub checked: Option<bool>,
}
