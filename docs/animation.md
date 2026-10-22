# Transitions and animations (M3d group 7)

How CSS motion works today. The code is [motion.rs](../crates/iron_ui/src/motion.rs) (timing,
interpolation, parsing, running state) and the restyle hooks in [tree.rs](../crates/iron_ui/src/tree.rs).
Browser differences are in [differences.md](differences.md). Animation of GameObjects (curves, clips,
`Animator`, skinning) is a separate system: see [animation-3d.md](animation-3d.md).

## Properties

`style.rs` keeps `Visual.transitions` (`TransitionLists`) and `Visual.animations` (`AnimationLists`).
Each is a set of comma lists (`transition-property`, `-duration`, `-delay`, `-timing-function`;
`animation-name`, `-duration`, `-delay`, `-timing-function`, `-iteration-count`, `-direction`,
`-fill-mode`, `-play-state`) combined by index when read (`specs()`). `parse_transition` and
`parse_animation` expand the shorthands into those lists. Inline camelCase props and stylesheets both
work.

## Timing and interpolation

- `Timing` is `linear`, the `ease*` keywords, `cubic-bezier()` or `steps()` (every jump position).
  `eval(t)` may overshoot 0..1 for bouncy curves.
- `interpolate(from, to, t)` blends **computed values as CSS text, token by token**. Numbers with the
  same unit blend; different units blend through `calc()`; colours blend premultiplied. Everything
  else must match exactly (same transform functions, same shadow layers, ...). `None` means "can't
  blend": transitions skip the property, animations flip halfway, as in CSS.
- `neutral` gives `none` a stand-in for a function list or shadow (zero translations, unit scales,
  transparent shadows), so `none` <-> `rotate(45deg)` works. `initial_value(prop)` is what a property
  has with no declaration; it returns `None` for `auto`, inherited and `currentColor` values.
- Text keeps this simple on purpose: no per-property interpolators, so new properties animate as soon as
  their computed text has blendable tokens.

## Restyle integration (`tree.rs`)

Motion runs at the end of each element's restyle, on top of the computed style:

1. Restyle records each property's **winning value** (`winners`), which transitions compare.
2. `@keyframes` for the element's animation names are resolved with `var()` and units applied.
3. `run_motion` applies animations first (`Animation::sample` gives a `Phase` and progress; fill mode,
   direction, iteration and pause are handled there; `keyframe_values` blends the surrounding frames,
   with the element's own value standing in for a missing 0% or 100%).
4. Then transitions: a property whose winning value changed and matches the new `transition` list
   starts a `Transition`. A change mid-flight starts from the current value (`value_at`). A property
   dropped from the list stops. Finished ones are dropped and reported.

`Motion` is the per-element running state (`values`, `transitions`, `animations`, `is_moving`).

Driving it over time:

- `set_time(ms)` marks every moving element dirty, so the next `draw_list` restyles it. `is_animating`
  tells the app to keep drawing frames while anything moves.
- `uses_motion` flips on once any element sets a `transition*`/`animation*`; from then on inline style
  changes also go through restyle (otherwise they skip it), so they can transition too.

## `@starting-style`

`css.rs` stores these rules in `Stylesheet::starting`; `tree.rs` indexes them (`starting_sheets`,
`starting_index`). A **fresh** element (not styled yet) or a **newly shown** one (e.g. a dialog gaining
`open`, or leaving `display: none`) has no old style, so its `@starting-style` declarations stand in
as the "before" values and its transitions run from them. An element that already had a style ignores
them, even if a class later adds a rule.

## `:outro` (deferred removal)

`:outro` is s&box's exit hook. `PseudoClass::Outro` matches `ElementState.outro`; `Deps::uses_outro` is
set when any selector uses it. Only then does removing a node by script keep it:

- The node moves to `leaving` with `outro` set; the restyle applies the `:outro` rules, which start
  transitions or animations.
- `remove_finished_outros` deletes it once the motion begun *since the exit started* (transitions and
  animations alike, by `start_ms >= since`) is done, or at once if none started. An infinite animation or a
  transition already running (an element removed mid fade-in) does not hold removal; an `:outro` that
  redirects a running transition restarts it, so that one does (`M3-ANI-05`).
- `is_leaving` (the node or an ancestor) hides it from hit testing, focus, Tab stops and modal
  trapping, and it is not displayed as a normal child.
- Without any `:outro` rule removal is immediate, as before.

## Events

Rust queues a `MotionEvent` (`kind`, `target`, `name`, `elapsed`) per change; `take_motion_events`
drains them. `Interaction::handle(Input::Refresh)` dispatches each as a `UiEvent` whose `value` is the
name (kebab-case property for transitions, animation name) and `delta_x` the elapsed seconds, skipping
nodes already removed.

| Kind | Types |
|---|---|
| `EventKind::Transition*` | `transitionrun` (created), `transitionstart` (past its delay), `transitionend`, `transitioncancel` (replaced or removed) |
| `EventKind::Animation*` | `animationstart`, `animationiteration`, `animationend`, `animationcancel` |

All bubble. React side: [react.md](react.md#events-eventsts).

## Tests

| ID | Where |
|---|---|
| M3-ANI-01 | unit tests in `motion.rs` (timing, interpolation, shorthand parsing) |
| M3-ANI-02 | `crates/iron_ui/tests/motion.rs` (transitions, events); `js/react/test/events.test.tsx` |
| M3-ANI-03 | unit tests in `motion.rs` and `css.rs` (`@keyframes` parsing); `tests/motion.rs` (running animations); `events.test.tsx` |
| M3-ANI-04 | `tests/motion.rs` (`@starting-style`, dialog open) |
| M3-ANI-05 | `tests/motion.rs` (`:outro` removal) |
