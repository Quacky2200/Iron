# Taffy 0.15.0: the tail of a float taller than the earlier floats is lost

`float_layout` (`compute/float.rs`, `FloatContext::place_floated_box`): when a float is placed starting in the
last segment and is taller than the segments left (the usual case for a short left float followed by a taller
right one), the placement search ends with `end = None` ("no segment can hold the rest"). The code then
takes `end_idx = segments.len() - 1` and updates only the existing segments' insets, so the part of the float
below the last segment is never recorded. `find_content_slot` then reports free width there, and later
blocks and text are laid out over the float.

Reproduction: a 300px block with `float: left` 80x40, then `float: right` 80x120, then text. Lines between
y=40 and y=120 should span x=0..220; without the fix they ran the full width, under the right float.
Test: `crates/iron_ui/tests/float.rs` `M3-FLT-15`, `M3-FLT-14`.

The fix (vendored in `third_party/taffy`, patched in through `[patch.crates-io]` in the root and `tools/`
`Cargo.toml`) pushes a segment for the tail in the `end = None` branch:

```rust
let end_y = start_y + floated_box.height;
if end_y > last_y_end {
    self.segments.push(Segment { y: last_y_end..end_y, insets: containing_block_insets, has_float: [false; 2] });
}
```

Drop the vendored copy once a Taffy release has this.
