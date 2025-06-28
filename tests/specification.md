# Markdown

This document is a list of all *markdown* features supported in the parser.

This mostly supports [commonmark](https://spec.commonmark.org/current/) (0.31.2 at the time of writing). [There are some things missing](https://github.com/kaleidawave/simple-markdown-parser/issues/3).

This is based on features supported by

- [GitHub](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax)
- [Markdoc](https://markdoc.dev/docs/syntax)
- [Obsidian](https://help.obsidian.md/obsidian-flavored-markdown)

Specifically these are supported extensions

- Tables
- Internal links (with block references)
- Comments (markdown)
- List task checkboxes
- Strikethroughs
- Highlights
- Callouts / alerts

%%
- Footnotes
- HTML elements
%%

> The output uses a custom `Debug` implementation to fully explore the parse output. See the documentation for the actual structure produced by the parser
> The returned text is based on `&str` referencing of the parse input.

## Elements

These are *top level items* in the document. These represent structural modifications, as apposed to individual styling.

### Headings

> [ATX Headings](https://spec.commonmark.org/0.31.2/#atx-headings)

Headings are specified by starting the line with the `#` symbol. More `#`s is a higher *depth*

```md
# Hi
## Hello
### Hiya

#not a heading
```

```
Heading { level: 1, content: "Hi" }
Heading { level: 2, content: "Hello" }
Heading { level: 3, content: "Hiya" }
Paragraph([Tag("not"), Plain(" a heading")])
```

> There are no level 7 headings
> Headings require a space between the `#` and content

#### Underscore headings

> [Setext Headings](https://spec.commonmark.org/0.31.2/#setext-headings)

```md
Hello
---

Text here
```

```
Heading { level: 1, content: "Hello" }
Paragraph("Text here")
```

### Paragraphs

Any *undecorated* content is considered paragraphs. Line breaks are considered *new paragraphs*

```md
This is text in a paragraph
```

```
Paragraph("This is text in a paragraph")
```

#### Split paragraphs

```md
This is text

Another paragraph
```

```
Paragraph("This is text")
Paragraph("Another paragraph")
```

#### Grouped paragraphs

```md
This is text in a paragraph
And another one
```

```
Paragraph([Plain("This is text in a paragraph"), Plain("And another one")])
```

#### Continued paragraphs

While by default the lines will be split a part. If the content ends with a backslash, the content will be continued

> [See](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#line-breaks)

```md
This is text \
in a paragraph
```

```
Paragraph([Plain("This is text "), LineBreak, Plain("in a paragraph")])
```

### Lists

Items can be visually *grouped* as a with `-`

```md
- something
- another thing
```

```
List([
	ListItem {
		enumerated: false, checked: None,
		inner: "something"
	},
	ListItem {
		enumerated: false, checked: None,
		inner: "another thing"
	}
])
```

#### Nesting

Using tab indentation we can add nest lists under lists

```md
- First item
	- Nested one
		- Deep
- Second item
	- One
	- Two
```

```
List([
	ListItem {
		enumerated: false, checked: None,
		inner: [
			Paragraph("First item"),
			List([
				ListItem {
					enumerated: false, checked: None,
					inner: [
						Paragraph("Nested one"),
						List([
							ListItem {
								enumerated: false, checked: None,
								inner: "Deep"
							}
						])
					]
				}
			])
		]
	},
	ListItem {
		enumerated: false, checked: None,
		inner: [
			Paragraph("Second item"),
			List([
				ListItem {
					enumerated: false, checked: None,
					inner: "One"
				},
				ListItem {
					enumerated: false, checked: None,
					inner: "Two"
				}
			])
		]
	}
])
```

#### Enumerating

We can prefix with increasing numerals for enumerated lists

```md
1. Hi
2. Something
3. X
```

```
List([
	ListItem {
		enumerated: true, checked: None,
		inner: "Hi"
	},
	ListItem {
		enumerated: true, checked: None,
		inner: "Something"
	},
	ListItem {
		enumerated: true, checked: None,
		inner: "X"
	}
])
```

#### Checkboxes

```md
- [x] Write specification
- [ ] Complete tests
```

```
List([
	ListItem {
		enumerated: false, checked: Some(true),
		inner: "Write specification"
	},
	ListItem {
		enumerated: false, checked: Some(false),
		inner: "Complete tests"
	}
])
```

#### Prefixes

> Discouraged and depreciated, what is wrong with good old dash `-`?

```md
* hi
* hello
```

```
List([
	ListItem {
		enumerated: false, checked: None,
		inner: "hi"
	},
	ListItem {
		enumerated: false, checked: None,
		inner: "hello"
	}
])
```

#### Containing stylings

```md
- **Bold**
- *Italic*
```

```
List([
	ListItem {
		enumerated: false, checked: None,
		inner: [Plain("Bold", bold)]
	},
	ListItem {
		enumerated: false, checked: None,
		inner: [Plain("Italic", emphasised)]
	}
])
```

#### Containing elements

```md
- List item
  > Quote item
```

```
List([
	ListItem {
		enumerated: false, checked: None,
		inner: [
			Paragraph("List item"),
			QuoteBlock {
				inner: "Quote item"
			}
		]
	}
])
```

### Code blocks

We can have blocks of literal content / code using triple (or more) backticks ```` ``` ````.

> Also nested (META!).

````md
```
code here
```
````

```
CodeBlock { indented_block: false, code: "code here" }
```

#### Languages

````md
```js
const x = 2;
```
````

```
CodeBlock { language: "js", indented_block: false, code: "const x = 2;" }
```

#### With indent

> Avoid

```md
    this is code
    avoid this syntax

    continued

Back to paragraph
```

```
CodeBlock { indented_block: true, code: "this is code\n    avoid this syntax\n\n    continued" }
Paragraph("Back to paragraph")
```

#### With tilda

```md
~~~js
this is code
avoid this syntax
~~~
```

```
CodeBlock { language: "js", indented_block: false, code: "this is code\navoid this syntax" }
```

### Mathematics blocks

We can have blocks of *mathematical* notation

```md
$$
y=\sin x
$$
```

```
MathematicsBlock(MathematicsBlock("y=\\sin x"))
```

> This is expected to go through a `LaTeX` or equivalent compiler

### Quotes

Prefixing with a item with `RIGHT-POINTING ANGLE BRACKET` marks the content as being quoted. Typically these are indented and have a colored left border

```md
> Hello
```

```
QuoteBlock {
	inner: "Hello"
}
```

#### Nesting

Content can be nested inside of quote blocks. It is important to note that this is **top-level** markdown and so can contain headers and even more quote blocks. Consequitive code blocks and combined in the end result

```md
> # Hello
> something
> > a quote in a quote
```

```
QuoteBlock {
	inner: [
		Heading { level: 1, content: "Hello" },
		Paragraph("something"),
		QuoteBlock {
			inner: "a quote in a quote"
		}
	]
}
```

#### Alerts

These are prefixes that can be added to quote blocks which relate to some custom stying and icon-age.

> First introduced by [GitHub](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts)

```md
> [!NOTE]
> Useful information that users should know, even when skimming content.

> [!TIP]
> Helpful advice for doing things better or more easily.

> [!IMPORTANT]
> Key information users need to know to achieve their goal.

> [!WARNING]
> Urgent info that needs immediate user attention to avoid problems.

> [!CAUTION]
> Advises about risks or negative outcomes of certain actions.
```

```
QuoteBlock {
	alert: "NOTE",
	inner: "Useful information that users should know, even when skimming content."
}
QuoteBlock {
	alert: "TIP",
	inner: "Helpful advice for doing things better or more easily."
}
QuoteBlock {
	alert: "IMPORTANT",
	inner: "Key information users need to know to achieve their goal."
}
QuoteBlock {
	alert: "WARNING",
	inner: "Urgent info that needs immediate user attention to avoid problems."
}
QuoteBlock {
	alert: "CAUTION",
	inner: "Advises about risks or negative outcomes of certain actions."
}
```

### Tables

Tables can be constructed in markdown through the use of delimiting cells with `|`. The table head is broken by a row or `| --- |` cells.

```md
| col1 | col2 |
| --- | --- |
| something | another |
| x | y |
```

```
Table([["col1", "col2"], ["something", "another"], ["x", "y"]])
```

#### Styling in tables

```md
| col1 | col2 |
| --- | --- |
| $something$ | another |
| x | **y** |
```

```
Table([["col1", "col2"], [[InlineMathematics("something")], "another"], ["x", [Plain("y", bold)]]])
```

### Horizontal rule

A rule. Can be used to divide up content

```md
some text

---
more text
```

> We have to have text here because otherwise it is treated as a frontmatter :/

```
Paragraph("some text")
HorizontalRule
Paragraph("more text")
```

### Block comments

```md
Some text
%%
some comment
%%
```

```
Paragraph("Some text")
CommentBlock("some comment")
```

### Frontmatter

> The parsing can be left to the user. But with the `yaml` feature [simple-yaml-parser](https://crates.io/crates/simple-yaml-parser) can be used to generate the below

```md
---
property: front_of_document
author:
	name: "Ben"
---

Content
```

```
Frontmatter { [Slice("property")] -> String("front_of_document"), [Slice("author"), Slice("name")] -> String("\"Ben\"") }
Paragraph("Content")
```

> Waiting for a fix

### Command blocks / custom blocks

> Called *tags* in [markdoc](https://markdoc.dev/docs/tags)

#### Inline

```md
{% image width=40 /%}
```

```
CommandBlock { name: image, arguments: [("width", "40")], inner: [] }
```

#### Blocks

```md
{% if true %}
Something here
{% /if %}
```

```
CommandBlock { name: if, arguments: [("", "true")], inner: [Paragraph("Something here")] }
```

### Block HTML elements

```md
<details>
<summary>Summary here</summary>

> Some markdown content
</details>

paragraph

<span class="inline">Inline span</span>

another paragraph

<div>
Something here
<div>
Inside thing
</div>
</div>

paragraph three
```

```
HTMLElement(HTMLElement("<details>\n<summary>Summary here</summary>\n\n> Some markdown content\n</details>"))
Paragraph("paragraph")
HTMLElement(HTMLElement("<span class=\"inline\">Inline span</span>"))
Paragraph("another paragraph")
HTMLElement(HTMLElement("<div>\nSomething here\n<div>\nInside thing\n</div>\n</div>"))
Paragraph("paragraph three")
```

#### HTML comments

```md
<!-- This is a comment -->

paragraph
```

```
HTMLElement(HTMLElement("<!-- This is a comment -->"))
Paragraph("paragraph")
```

## Styling

These are inline elements

There are blocks inside markdown

- Links
- Text

These can be decorated, but whose decoration be altered until finished. The decoration may or may not apply to the item

- Code
- Mathematics
- Tags (from Obsidian)
- Interpolations
- Emoji

The rest is considered regular text, but can be styled with the following. These are binary, e.g. you cannot have double bold

- Emphasis (italic)
- Bold
- Crossout
- Highlight
- Superscript
- Subscript

> No underlines, colors

### Code

```md
We have some `code` here
```

```
Paragraph([Plain("We have some "), InlineCode("code"), Plain(" here")])
```

#### Nesting

We can use backticks in `code` by wrapping the whole thing in more backticks

```md
`` `hi` ``
```

> Currently trimmed

```
Paragraph([InlineCode("`hi`")])
```

> Unfortuantly, I don't think we can remove the spaces here?

#### Escaping

We can use backticks in regular markdown by escaping the content with a backslash. `\`

```md
My favorite character: \` the backtick
```

```
Paragraph("My favorite character: \\` the backtick")
```

### Emphasis

> This corresponds to the `<em>` tag which italicizes text

```md
This is *emphasised* text
```

```
Paragraph([Plain("This is "), Plain("emphasised", emphasised), Plain(" text")])
```

### Bold

```md
This is **bold** text
```

```
Paragraph([Plain("This is "), Plain("bold", bold), Plain(" text")])
```

#### Bold in emphasis

```md
This is **bold and _emphasised_** text
```

```
Paragraph([Plain("This is "), Plain("bold and ", bold), Plain("emphasised", bold, emphasised), Plain(" text")])
```

#### Bold and emphasised

```md
This is ***bold and emphasised*** text
```

```
Paragraph([Plain("This is "), Plain("bold and emphasised", bold, emphasised), Plain(" text")])
```

#### Bold in code

```md
This is **bold `return 0`** text
```

```
Paragraph([Plain("This is "), Plain("bold ", bold), InlineCode("return 0", bold), Plain(" text")])
```

### Links

Text can be linked. The content is given in square brackets `[...]` followed by a reference in parenthesis `(...)`.

```md
[title](https://google.com)
```

```
Paragraph([ExternalLink { to: "https://google.com" } ("title")])
```

#### Styles in links

The content/text of a link can have styles

```md
[something *here*](https://google.com)
```

```
Paragraph([ExternalLink { to: "https://google.com" } ("something *here*")])
```

#### Internal links

```md
[#x]
```

```
Paragraph([InternalLink { to: "#x" } ("#x")])
```

#### Media links

> Here the text content becomes its [`alt` text](https://developer.mozilla.org/en-US/docs/Web/API/HTMLImageElement/alt)

```md
![...](https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExeHhzc2tuNmFlcHNnMWRyNG1jNzlkMXA3andraGRvZDh2MzJ4cXJvcSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/SwpQtSRf8Ekby5IsTw/giphy.gif)
```

```
Paragraph([MediaLink { source: "https://media1.giphy.com/media/v1.Y2lkPTc5MGI3NjExeHhzc2tuNmFlcHNnMWRyNG1jNzlkMXA3andraGRvZDh2MzJ4cXJvcSZlcD12MV9pbnRlcm5hbF9naWZfYnlfaWQmY3Q9Zw/SwpQtSRf8Ekby5IsTw/giphy.gif", alt: "..." } ])
```

#### Chevron links

```md
Link to <https://github.com/kaleidawave/benchmarks>
```

```
Paragraph([Plain("Link to "), ExternalLink { to: "https://github.com/kaleidawave/benchmarks" } ("https://github.com/kaleidawave/benchmarks")])
```

### Inline mathematics

Similar to [#block-mathematics] we have inline mathematics

```md
Euler's criterion $\left({\frac{a}{p}}ight)\equiv a^{\tfrac {p-1}{2}}{\pmod {p}}$ a formula for determining whether an integer is a quadratic residue modulo a prime
```

```
Paragraph([Plain("Euler's criterion "), InlineMathematics("\\left({\\frac{a}{p}}ight)\\equiv a^{\\tfrac {p-1}{2}}{\\pmod {p}}"), Plain(" a formula for determining whether an integer is a quadratic residue modulo a prime")])
```

### Strikethrough

Two tildas `~` cross out certain text.

```md
~~cross this out~~
```

```
Paragraph([Plain("cross this out", strikethrough)])
```

### Tags

> This is [based of an Obsidian feature](https://help.obsidian.md/tags)

```md
We can #tag this
```

```
Paragraph([Plain("We can "), Tag("tag"), Plain(" this")])
```

### Emoji

```md
I hope this works :smile:
```

```
Paragraph([Plain("I hope this works "), Emoji("smile")])
```

### Superscript and subscript

```md
The 16^th^ of November

Ozone layer O~3~
```

```
Paragraph([Plain("The 16"), Plain("th", superscript), Plain(" of November")])
Paragraph([Plain("Ozone layer O"), Plain("3", subscript)])
```

### Inline HTML elements

```md
Some text with <span id='workaround'>text</span> here <!-- comment here -->
```

```
Paragraph([Plain("Some text with "), HTMLElement(HTMLElement("<span id='workaround'>text</span>")), Plain(" here "), HTMLElement(HTMLElement("<!-- comment here -->"))])
```

### Interpolation

> For supporting MDX

```md
The day is {date}.
```

```
Paragraph([Plain("The day is "), Interpolation("date"), Plain(".")])
```

### External link content

```md
![LOC badge](https://project-information-kaleidawave.val.run/project/simple-markdown-parser/badge)
[![crates.io badge](https://img.shields.io/crates/v/simple-markdown-parser?style=flat-square)](https://crates.io/crates/simple-markdown-parser)
[![docs.rs badge](https://img.shields.io/docsrs/simple-markdown-parser?style=flat-square)](https://docs.rs/simple-markdown-parser/latest)
```

```
Paragraph([MediaLink { source: "https://project-information-kaleidawave.val.run/project/simple-markdown-parser/badge", alt: "LOC badge" } , ExternalLink { to: "https://crates.io/crates/simple-markdown-parser" } ("![crates.io badge](https://img.shields.io/crates/v/simple-markdown-parser?style=flat-square)"), ExternalLink { to: "https://docs.rs/simple-markdown-parser/latest" } ("![docs.rs badge](https://img.shields.io/docsrs/simple-markdown-parser?style=flat-square)")])
```
