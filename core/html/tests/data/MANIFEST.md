# `core/html` support manifest

The declared cut of `PRD-008` and `docs/v0-5-handoff/02-b5-html-tokenizer.md`, tag by tag and syntactic construct by
construct.

`core/html/tests/manifest_runner.rs` checks three things and fails loudly on any of them:

1. this file and `html::SUPPORTED_TAGS` / `html::SUPPORTED_SYNTAX` name the same sets, **in both directions**;
2. every listed token has a probe that the parser really accepts, and correctly constructs in the `dom::DomTree`;
3. a battery of invalid forms is safely handled or refused with typed errors.

There is **no bless path**: this file is hand-maintained, because the `notes` column carries reasons no generator can
invent. A new supported token means editing the registry in `core/html/src/lib.rs`, this table, and the probe table in
the runner — all three, or CI is red.

`since` is the phase of the roadmap that landed the token (B5 for this v0.5 cut).

## Tags

The HTML elements the parser recognizes and constructs into DOM element nodes.

| token        | since | notes                                                        |
| ------------ | ----- | ------------------------------------------------------------ |
| `a`          | B5    | inline hyperlink element                                     |
| `article`    | B5    | block sectioning container                                   |
| `blockquote` | B5    | block quotation container                                    |
| `body`       | B5    | main document body container                                 |
| `br`         | B5    | void line break element                                      |
| `code`       | B5    | inline code span                                             |
| `div`        | B5    | block division container                                     |
| `em`         | B5    | inline emphasis element                                      |
| `footer`     | B5    | block footer container                                       |
| `h1`         | B5    | level 1 section heading                                      |
| `h2`         | B5    | level 2 section heading                                      |
| `h3`         | B5    | level 3 section heading                                      |
| `h4`         | B5    | level 4 section heading                                      |
| `h5`         | B5    | level 5 section heading                                      |
| `h6`         | B5    | level 6 section heading                                      |
| `head`       | B5    | document metadata container                                  |
| `header`     | B5    | block header container                                       |
| `hr`         | B5    | void thematic break element                                  |
| `html`       | B5    | root document element                                        |
| `img`        | B5    | void embedded image element                                  |
| `li`         | B5    | list item element with auto-closing tag omission             |
| `link`       | B5    | void document resource link metadata                         |
| `main`       | B5    | block main content container                                 |
| `meta`       | B5    | void document metadata element                               |
| `nav`        | B5    | block navigation container                                   |
| `noscript`   | B5    | alternate content container                                  |
| `ol`         | B5    | ordered list block container                                 |
| `p`          | B5    | paragraph block element with auto-closing tag omission       |
| `pre`        | B5    | preformatted text container                                  |
| `script`     | B5    | script element; content tokenized as RAWTEXT, not markup     |
| `section`    | B5    | block generic section container                              |
| `span`       | B5    | inline text container                                        |
| `strong`     | B5    | inline strong importance element                             |
| `style`      | B5    | stylesheet element; content tokenized as RAWTEXT, not markup |
| `title`      | B5    | document title metadata element                              |
| `ul`         | B5    | unordered list block container                               |

## Syntax

Syntactic constructs handled by the HTML5 tokenizer state machine and tree builder.

| token                       | since | notes                                                                         |
| --------------------------- | ----- | ----------------------------------------------------------------------------- |
| `<!DOCTYPE html>`           | B5    | standard HTML5 document type declaration                                      |
| `<tag attr="val">`          | B5    | double-quoted attribute syntax                                                |
| `<tag attr='val'>`          | B5    | single-quoted attribute syntax                                                |
| `<tag attr=val>`            | B5    | unquoted attribute syntax                                                     |
| `<tag bool-attr>`           | B5    | boolean attribute syntax without value                                        |
| `<tag />`                   | B5    | self-closing tag syntax                                                       |
| `<!-- comment -->`          | B5    | HTML comment syntax                                                           |
| `&entity; named entity`     | B5    | named character reference entity resolution                                   |
| `&#decimal; numeric entity` | B5    | decimal character reference resolution                                        |
| `&#xhex; numeric entity`    | B5    | hexadecimal character reference resolution                                    |
| `<script> rawtext`          | B5    | rawtext mode in `<script>`: inner markup is never parsed as HTML elements     |
| `<style> rawtext`           | B5    | rawtext mode in `<style>`: inner CSS markup is never parsed as HTML elements  |
| `p tag omission`            | B5    | open `<p>` is implicitly closed before opening another `<p>` or block element |
| `li tag omission`           | B5    | open `<li>` is implicitly closed before opening another `<li>`                |
| `void tags auto-close`      | B5    | void tags do not trap subsequent elements as children                         |
