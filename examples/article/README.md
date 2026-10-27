# article

A rich-text style article built from plain boxes, mixing floats and a table: a picture floating left and
a pull-quote floating right with the text wrapping around both, a sortable grid table between
paragraphs, and a second picture floated right further down. Headings use `clear: both` to end the wrap.
This is the case a separate "richtext" element was considered for; it needs none (see `docs/ui-tree.md`).

APIs used: `float`, `clear`, `display: grid` with `display: contents` rows, `Image`, `Text`, `useState`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/article
```

Then from the repo root (`cd ..`):

```bash
cargo run -p iron_app --release -- examples/article --screenshot out.png --frames 30
```
