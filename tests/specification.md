# Markdown

This document is a list of all markdown features

## Elements

These are *top level items* not nested items

### Headings

Headings are specified by starting the line with the `#` symbol. More `#`s is a higher *depth*

```md
# Hi
## Hello
### Hiya
```

```
Heading { level: 1, parts: ["Hi"] }
Heading { level: 1, parts: ["Hi"] }
Heading { level: 1, parts: ["Hi"] }
```

> There are no level 7 headings?

### Paragraphs

Any *undecorated* content is considered paragraphs. Line breaks are considered *new paragraphs*

```md
This is text in a paragraph
And another one
```

```
Paragraph { parts: ["This is text in a paragraph"] }
Paragraph { parts: ["And another one"] }
```

### Lists

Items can be visually *grouped* as a with `-`

```md
- something
- another thing
```

```
```

#### Nesting

```md
- something
	- another thing
		- z
- Something
	- x
	- y
```

```
```

> TODO invalid syntax here

#### Enumerating

```md
1. Hi
2. Something
3. X
```

```
```

#### Checkboxes

```md
- [x] Write specification
- [ ] complete tests
```

```
```

#### Prefixes

> Discouraged and depreciated, what is wrong with good old dash `-`?

```md
* hi
* hello
```

```
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
Code { language: "", code: "" }
```

#### Languages

````md
```js
const x = 2;
```
````

```
Code { language: "js", code: "" }
```

### LaTeX / Math blocks

We can have blocks of *mathematical* notation 

```md
$$
y=\sin x
$$
```

```
Mathematics { content: "y=\sin x" }
```

> This is expected to go through a LaTeX or equivalent compiler

### Quotes

> Nested here

```md
> Hello
```

#### Nesting

```md
> # Hello
> something
> > a quote in a quote
```

```
```

### Tables

```md
| col1 | col2 |
| --- | --- |
| something | another |
| x | y |
```

```
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
```

### Footnotes

> TODO

### Block comments

```md
Some text
%%
some comment
%%
```

```
```

## Styling

These are inline elements

### HTML elements

> This is based of [lightml](https://github.com/kaleidawave/lightml)

### Code

```md
We have some `code` here
```

```
```

#### Nesting

We 

```md
`` `hi` ``
```

```
```

### Emphasis / italic

```md
This is *emphasised* text
```

```
```

### Bold

```md
This is **bold** text
```

```
```

#### Bold in italics

### Links (all of them)

```md
[title](https://google.com)
```

```
```

### LaTeX

```md
Euler's criterion $\left({\frac{a}{p}}\right)\equiv a^{\tfrac {p-1}{2}}{\pmod {p}}$
```

```
```

### Strikethrough

```md
~~cross this out~~
```

```
```

### Tags

> This is based of an Obsidian feature

```md
We can #tag this
```

```
```

### Emoji

```md
We can :smile:
```

```
```

### Superscript and subscript

```md
The 16^th of November
```

```
```

## Others

### Frontmatter

> The parsing of the content is left to the user. Can I recommend [simple-yaml-parser](https://crates.io/crates/simple-yaml-parser)?

```md
---
property: front_of_document
---

Content
```

```
```

### Custom blocks

> #TODO Stripe..? 

### Comments

> #TODO
