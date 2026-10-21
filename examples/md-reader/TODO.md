# md-reader: TODO

Legend: `[ ]` to do · `[~]` in progress · `[x]` done

## Done

- [x] Docs bundled at build time (`gen-docs.ts`): `docs/` and the top-level `.md` files, images
  copied into the game folder
- [x] Markdown parser: headings, paragraphs, emphasis, inline code, links, images, nested and
  task lists, quotes, GitHub tables, rules, code blocks; it scans with sticky patterns rather
  than copying the rest of each line (TODO.md: 293 ms to 36 ms in QuickJS)
- [x] File tree on the left, toggled from the toolbar (☰, Ctrl+B)
- [x] Back / forward (buttons, Alt+← / Alt+→, mouse side buttons), keeping each page's scroll
- [x] Home button (an icon)
- [x] Links between docs and to `#headings`; link targets in the status bar
- [x] Inline links take clicks and `:hover` (spans are hit-tested in `iron_ui`, `M3-EVT-07`)
- [x] Images (PNG, JPEG, SVG); videos get a placeholder
- [x] Path bar like Explorer's: each folder opens a listing of that folder, "Iron" the root
- [x] Long pages load progressively: a screenful at once, then a chunk a frame (memoized chunk
  boxes, long lists cut between items); anchors and scroll restores mount what they need first
- [x] Engine work it prompted (`iron_ui`): an exact size cache beyond Taffy's nine entries
  (`M2-LAY-07`), drawing culled to what scroll containers show (`M2-DRW-09`), line breaking
  skipped when a block's width is unchanged. A scroll frame of TODO.md: ~21 ms to ~5 ms
  (release)
- [x] Syntax highlighting: TypeScript / JavaScript, Rust, WGSL, CSS, JSON, TOML, shell (One Dark)
- [x] Light / dark theme toggle on the right of the toolbar, remembered in `PlayerPrefs`
  (light code colours: One Light)

## Next

- [ ] Jumps land a little off when the page has only just mounted: the first layout of new
  content and the next one disagree, so the heading's position moves after the jump (seen at
  "6.2 fetch" in `docs/research/networking-and-files.md`). Fix the layout so one pass is final
- [ ] More languages for highlighting: Python, C / C++, C#, GLSL / HLSL, HTML / XML, YAML,
  Markdown, diff, PowerShell (its own rules rather than shell's), SQL
- [ ] Fuzzy search for files in the current folder (and below), keyboard-first (Ctrl+P)
- [ ] Find in page (Ctrl+F) with matches highlighted, next / previous, a match count
- [ ] Command-line arguments (needs Iron to pass extra arguments to scripts, and runtime file
  reading for `--root`, M6 Script IO):
  - [ ] `--root <folder>`: the folder of docs to read instead of Iron's own
  - [ ] `--goto "<doc>#<heading-id>"`: open a doc at a heading; the doc path is relative to the
    root. `#L<line>` jumps to a line (and highlights it)
  - [ ] The same `doc#fragment` links from outside, as a browser takes a URL with a fragment:
    jump there and highlight the target briefly
- [ ] An "on this page" outline, selectable / copyable text and code
- [ ] Open external links in the system browser (needs an app shell op)
- [ ] GIF images (Iron decodes PNG, JPEG and SVG today; see the root TODO.md)
- [ ] Scrolling still lays out again on every wheel tick (two passes, ~55 ms a frame in a debug
  build): move the scrolled subtree's rects instead; consider building `iron_ui` with some
  optimisation in dev builds
