# Native animation (`iron_animation`)

Animation without a script engine. Design and review: [research/iron-native-animation.md](research/iron-native-animation.md).
Behaviour and numbers follow `js/engine/src/animation.ts`, which stays as the oracle for the tests.

## Crates

- `crates/iron_animation`: curves (`curve.rs`), clips and the arenas (`clip.rs`), pose blending (`pose.rs`), the `Animation`
  component (`player.rs`), controller data (`controller.rs`), the `Animator` (`animator.rs`) and the property registry
  (`registry.rs`). No script engine in it.
- `crates/iron_engine_api/src/animation.rs`, `animator.rs`: the script API (`#[iron_api]`), for compiled code and QuickJS.

## Frame

The host calls `iron_engine_api::animation::update(dt, other)` after the components' updates. Players go depth first through the
scene (inactive objects skipped); each blends its clips into one `Pose` (weighted sums per transform channel, quaternions
sign-aligned then normalised, other properties weighted averages) and writes it once. Clip events that came due are returned
as `FiredEvent { object, function_name, parameter }` for the host to deliver to the object's components (and kept for
`Animations.takeEvents()`). `other` is a `PropertySink` for components the registry does not know (a game's compiled ones).

## Script API

| Class | Made by | Notes |
|---|---|---|
| `Keyframe` | `new Keyframe(time, value, inTangent?, outTangent?)` | value class |
| `AnimationCurve` | `new AnimationCurve()`, `.linear/easeInOut/constant(..)` | `addKey(time, value?)`, `addKeyframe(k)`, `removeKey`, `key(i)`, `evaluate(t)`, `length`, `preWrapMode`, `postWrapMode` |
| `AnimationClip` | `new AnimationClip(name?)` | `setCurve(path, "Transform", "localPosition.x", curve)`, `removeCurve`, `addEvent(time, fn, parameter?)`, `wrapMode`, `frameRate`, `length`, `sampleAnimation(go, t)`, `toJson()` / `AnimationClip.fromJson(text)` |
| `Animation` | `go.addComponent(Animation)` | `addClip`, `play`, `crossFade`, `stop`, `isPlaying`, `state(name)` (`AnimationState`: `time`, `speed`, `weight`, `enabled`, `normalizedTime`), `clip`, `playAutomatically` |
| `AnimationParameter`, `AnimationCondition`, `AnimationTransition`, `AnimationBlendTree`, `AnimationNode` | `new X(..)` with `{ option }` setters | the pieces of a controller; `AnimationNode.blend(name, tree)` |
| `AnimationController` | `new AnimationController()` or `AnimationController.build()...create()` | `addParameter/addNode/addTransition`, `defaultState`, counts and `...At(i)`, `toJson` / `fromJson` |
| `AnimationBuilder` | `AnimationController.build()` | chained: `parameter`, `state`, `blendState` + `child`, `transition` + `when` / `exitTime` / `duration` / `offset`, `defaultState`, `create` |
| `Animator` | `go.addComponent(Animator)` | `controller`, `speed`, `set/getFloat/Integer/Bool`, `setTrigger`, `resetTrigger`, `play`, `crossFade`, `isInTransition`, `currentStateName` / `currentNormalizedTime` / `currentLength` and `next...` |

Targets of a curve: `Transform` (`localPosition.x|y|z`, `localScale.x|y|z`, `localRotation.x|y|z|w`, `localEulerAngles.x|y|z`),
`GameObject` (`active`) and registered components (`Camera`: `fieldOfView`, `nearClipPlane`, `farClipPlane`, `depth`,
`orthographicSize`, `orthographic`, `backgroundColor.r|g|b|a`). Register more with `iron_animation::register_type` and
`register_setter`.

An `Animator` state keeps its normalized time (Unity's), not seconds: a blend tree's length changes with its weights, so the
phase stays put when they change (a walk breaking into a run carries on mid-stride) and then advances at the blend's pace
(`M5-ANI-07`). `iron_animation::gltf` turns a glTF animation's channels into a looping clip (`Model.animations`), each rotation
key flipped to its predecessor's side (`q` and `-q` are one turn; mixed sides sampled between them pass near zero, a wild pose
for a frame), cubic keys flipping with their tangents (`M5-SKN-04`). `Animation.destroy()` empties the component's slot (players
are numbered by place).

## Differences from `animation.ts`

Object literals became classes and a builder; a clip target is a class-name string; an event parameter is a number; JSON is text;
`addKey` has no tangents (`addKeyframe` does); `fromPoints` waits for arrays across the boundary; a parameter default is a number
(the controller's `addParameter(name, type, true)` takes a flag too);
`getCurrentAnimatorStateInfo()` is properties; the default clip starts in the first update and events are delivered after the
frame's pose is written; `Animator` objects update after `Animation` objects on the same object. Tests `M9-TS-676..697`.
