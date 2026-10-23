# Native animation (`iron_animation`)

Animation without a script engine. Design and review: [research/iron-native-animation.md](research/iron-native-animation.md).
Behaviour and numbers follow `js/engine/src/animation.ts`, which stays as the oracle for the tests.

## Crates

- `crates/iron_animation`: curves (`curve.rs`), clips and the arenas (`clip.rs`), bindings resolved per animated root
  (`bind.rs`), pose blending (`pose.rs`), the `Animation` component (`player.rs`), controller data (`controller.rs`), the
  `Animator` (`animator.rs`), culling (`culling.rs`), bench timers (`profile.rs`) and the property registry (`registry.rs`).
  No script engine in it.
- `crates/iron_engine_api/src/animation.rs`, `animator.rs`: the script API (`#[iron_api]`), for compiled code and QuickJS.

## Frame

The host calls `iron_engine_api::animation::update(dt, other)` after the components' updates. Players go depth first through the
scene (inactive objects skipped); each blends its clips into one `Pose` (weighted sums per transform channel, quaternions
sign-aligned then normalised, other properties weighted averages) and writes it once. Clip events that came due are returned
as `FiredEvent { object, function_name, parameter }` for the host to deliver to the object's components (and kept for
`Animations.takeEvents()`). `other` is a `PropertySink` for components the registry does not know (a game's compiled ones).
The compiled-game host calls `update_culled(dt, other, visible)` instead (see Culling).

**Bindings (Unity's `Animator.Rebind`).** A clip's curves are resolved once per (clip, animated root) (`bind.rs`): each
`Transform` curve gets a bone slot (an index into the root's bone table) and a channel number (position x y z, rotation
x y z w, scale x y z, Euler x y z; a property that is none of them only touches its bone, as before), each component curve
its object and an interned (class, property) key, and each bone's quaternion curves are listed for aligning blends. A root's
bindings are kept against `Scene::tree_version(root)`, a number that changes when a name or a parent link changes anywhere
under the object (a rename, a re-parenting, a new child, a destroyed object; moves and flags don't touch it), and a clip's
against `World`'s edit count (`clip_mut` / `curve_mut` bump it). Roots that are gone are swept once their number doubles.
`Pose` is then a dense per-bone array of sums and weights (no strings): `add_aligned` samples every curve once and takes each
bone's dot product from those values; curves of one bone with the same key times and wrap modes (a glTF channel's x, y, z,
w) share one key search (`Curve::locate`, then `at`; `evaluate` is the two together, so values are unchanged); rotations
are still blended component-wise and normalised (nlerp). Players fill one scratch `Pose` kept in the `World`; `apply`
writes each touched bone once and empties it, keeping its buffers. Clip lengths are kept until a clip or curve changes or
a curve is added. The output is the same bit for bit as the string-keyed `Pose` it replaced, which stays as a test oracle
(`legacy_pose.rs`, `M5-ANI-08`). `Scene::set_local` marks moved objects through a per-object flag and stops at objects
already stale and listed (`M5-OBJ-10`).

**Culling (Unity's `Animator.cullingMode`, `Animation.cullingType`).** `Animator.cullingMode` is the enum
`AnimatorCullingMode`: `alwaysAnimate` (the default: as before, visibility is never asked), `cullUpdateTransforms` (while
unseen the state machine, clock, parameters and events go on but no transform or component property is written, so once
seen the pose is where it would have been, `M5-ANI-09`, `M5-ANI-11`) and `cullCompletely` (while unseen nothing runs, no
events fire and the clock stops; it holds its last pose and resumes from there, `M5-ANI-10`). `Animation.cullingType` is
`AnimationCullingType`: `alwaysAnimate` or `basedOnRenderers` (paused while unseen, like `cullCompletely`, `M5-ANI-12`).
*Seen* means any renderer on the animated object or below it was drawn last frame: inside a camera's frustum (and not past
its fog's cull distance), or a shadow caster in a light's shadow map whose shadow can fall in a view (the box swept along
a directional light meets the frustum; for spot and point lights, the light reaches the frustum and the box is in its
range), so a figure off screen whose shadow is on screen keeps animating. An object with no renderers is always seen.
How it is wired: `culled_roots()` lists the objects whose components cull; the app (`crates/iron_app/src/anim_cull.rs`)
watches only the renderers under them, giving their meshes a `MeshInstance::visibility_key`; `WorldRenderer::seen()` /
`take_seen()` reports which were seen (skinned meshes judged by their bind-pose box under every bone, a box that holds
them); the next frame's update asks through `iron_animation::Visibility` (`M5-ANI-13`, `M5-CUL-03`). This is one frame
late. Cameras of secondary windows are not counted. Colliders and objects attached to bones are not moved while culled.

**Bench timers.** With `--bench`, `profile.rs` adds `animation: update (all)`, `animation: pose add`, `animation: pose
apply` and `animation: set_local` to `syncParts` (`sync_profile` merges them); each is a clock read per call, and
`set_local` is timed per bone, so its own share is partly the timer.

## Script API

| Class | Made by | Notes |
|---|---|---|
| `Keyframe` | `new Keyframe(time, value, inTangent?, outTangent?)` | value class |
| `AnimationCurve` | `new AnimationCurve()`, `.linear/easeInOut/constant(..)` | `addKey(time, value?)`, `addKeyframe(k)`, `removeKey`, `key(i)`, `evaluate(t)`, `length`, `preWrapMode`, `postWrapMode` |
| `AnimationClip` | `new AnimationClip(name?)` | `setCurve(path, "Transform", "localPosition.x", curve)`, `removeCurve`, `addEvent(time, fn, parameter?)`, `wrapMode`, `frameRate`, `length`, `sampleAnimation(go, t)`, `toJson()` / `AnimationClip.fromJson(text)` |
| `Animation` | `go.addComponent(Animation)` | `addClip`, `play`, `crossFade`, `stop`, `isPlaying`, `state(name)` (`AnimationState`: `time`, `speed`, `weight`, `enabled`, `normalizedTime`), `clip`, `playAutomatically`, `cullingType` |
| `AnimationParameter`, `AnimationCondition`, `AnimationTransition`, `AnimationBlendTree`, `AnimationNode` | `new X(..)` with `{ option }` setters | the pieces of a controller; `AnimationNode.blend(name, tree)` |
| `AnimationController` | `new AnimationController()` or `AnimationController.build()...create()` | `addParameter/addNode/addTransition`, `defaultState`, counts and `...At(i)`, `toJson` / `fromJson` |
| `AnimationBuilder` | `AnimationController.build()` | chained: `parameter`, `state`, `blendState` + `child`, `transition` + `when` / `exitTime` / `duration` / `offset`, `defaultState`, `create` |
| `Animator` | `go.addComponent(Animator)` | `controller`, `speed`, `cullingMode`, `set/getFloat/Integer/Bool`, `setTrigger`, `resetTrigger`, `play`, `crossFade`, `isInTransition`, `currentStateName` / `currentNormalizedTime` / `currentLength` and `next...` |

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
