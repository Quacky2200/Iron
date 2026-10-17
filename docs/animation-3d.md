# Animation of objects (curves, clips, Animator, skinning)

How animation of GameObjects works today. (CSS motion is a different system: see
[animation.md](animation.md).) The code is [animation.ts](../js/engine/src/animation.ts), with
`__iron_animate` called from [components.rs](../crates/iron_script/src/components.rs), glTF skins and
animations in [models.rs](../crates/iron_app/src/models.rs) and [models.ts](../js/engine/src/models.ts),
and GPU skinning in [world/mod.rs](../crates/iron_render/src/world/mod.rs). Demo:
[examples/animation](../examples/animation/src/main.tsx). The API follows Unity's.

All of the animation logic runs in scripts. Rust only calls `__iron_animate` once a frame and, for
skinned meshes, turns bone objects into matrices for the GPU.

## Curves and clips

- `Keyframe(time, value, inTangent, outTangent)`: tangents are slopes (value per second).
- `AnimationCurve` joins keys with cubic Hermite segments shaped by their tangents. Infinite tangents
  make steps. Before the first key and after the last, a wrap mode applies (`once`, `loop`, `pingPong`,
  `clampForever`; the curve default is `clampForever`). `linear`, `easeInOut`, `constant` and
  `fromPoints(points, "linear" | "smooth" | "step")` build common shapes; `evaluate(t)` samples.
- `AnimationClip` holds curves by **relative path** (`""` is the animated object, `"Arm/Hand"` a
  descendant by names), target and property, set with `setCurve(path, target, property, curve)`:
  - `"Transform"`: `localPosition.x|y|z`, `localScale.x|y|z`, `localRotation.x|y|z|w` (a quaternion,
    normalized after blending) or `localEulerAngles.x|y|z`.
  - A component class (or its name): any number property, or one field of a value (`color.r`,
    `size.x`). Booleans count as on above 0.5.
  - `"GameObject"`, `active`: turns the object on and off.
  - `length`, `wrapMode`, `sampleAnimation(go, time)` (poses an object at a time), and `toJSON` /
    `fromJSON`. A missing path or component is skipped.
- Clip events (`addEvent({ time, functionName, parameter? })`) call a method on the animated object's
  components when playback passes `time` (looping too, once per pass).

## Blending (`Pose`)

Players do not write objects directly. Each frame a `Pose` collects every clip's contribution with its
weight: transform channels are weighted sums, and rotations are sign-aligned (the short way round) and
normalized. `apply()` then writes each object once: all of its transform channels go out in one
`Transform._setLocal`, which sends a single `scene.local` op. Other properties are set on their
components, so setters run as usual.

## `Animation` (legacy player)

A component with `clip`, `clips` and `playAutomatically`. `play(name)`, `crossFade(name, seconds)`,
`stop(name?)`, `isPlaying`, `addClip` and `state(name)` (an `AnimationState`: `time`, `speed`, `weight`,
`wrapMode`, `normalizedTime`). A clip played once holds its last pose and stops. Cross-fades move weights
linearly; halfway is an even blend.

## `Animator` and `AnimatorController`

An `AnimatorController` is one layer of `parameters` (`float`, `int`, `bool`, `trigger`), `states` and
`transitions`, built with `addParameter`, `addState` and `addTransition` and saved as JSON.

- A state's motion is a clip or a **1D blend tree** (`blendParameter` and children with `threshold`
  and `timeScale`): the two clips around the parameter's value blend, and share the state's
  normalized time. A state can have `speed` and a `speedParameter` multiplying it.
- A transition goes `from` a state name or `"Any"` (not into itself) `to` a state. It fires when its
  conditions hold (`if`, `ifNot`, `greater`, `less`, `equals`, `notEqual`) and, with `hasExitTime`,
  once the source passes `exitTime` (normalized). `duration` is the cross-fade in seconds (0 switches at
  once) and `offset` where the destination starts. Triggers used by a transition reset when it fires.
  The first matching transition wins, and none starts while another is fading.
- `Animator` sets parameters (`setFloat`, `setInteger`, `setBool`, `setTrigger`, `resetTrigger` and the
  getters), jumps with `play(state, normalizedTime)` / `crossFade(state, duration)`, and reports
  `isInTransition` and `getCurrentAnimatorStateInfo()` / `getNextAnimatorStateInfo()`.
- While fading, both states are posed with weights `1 - w` and `w`. Events fire for a clip while its
  weight is at least half.

## When it runs

`ComponentHost::update_active` runs `onUpdate` for all components, then calls `__iron_animate(dt)` (if
the bundle has it), then `onLateUpdate`. So scripts set Animator parameters in `onUpdate`, the pose is
written, and late updates (a camera following a bone) read it. `__iron_animate` walks the active
objects and advances each `Animation` and `Animator`; a player that throws is logged and the others
continue. `dt` is the scaled frame time.

## Scenes

`Animation` and `Animator` are built-in components. Curves, clips and controllers inside properties are
saved as `$type` values (`AnimationCurve`, `AnimationClip`, `AnimatorController`) and read back by
`scenes.ts`, so scenes and prefabs keep them.

## Skinned meshes

A skinned mesh moves each vertex with up to four **bones** (GameObjects), weighted. The engine holds
only ids: scripts say which objects are the bones and Rust makes the matrices.

**Scripts.** `Mesh.create({ vertices, triangles, boneWeights, bindposes })` takes per-vertex
`BoneWeight`s (`indices` and `weights`, normalized on send) and each bone's inverse bind matrix.
`SkinnedMeshRenderer` (a `MeshRenderer`) has `bones` (in the order the weights index them) and
`rootBone`. The mesh op carries `joints` and `weights` (4 per vertex) after the indices; the renderer op
carries the bone object ids and the bind poses, 16 numbers each. The renderer's own transform does not
apply: bones are in world space. Animate the bones with an `Animator` or `Animation` and the mesh bends.

**In the app.** `WorldViews` keeps a `Skin { bones, bindposes }` per renderer (`skins`). Each frame
`views()` makes one matrix per bone, `world matrix of the bone object * inverse bind pose` (a bone object
that is gone holds its bind pose), appends them to `WorldView::bones` and records each renderer's range
in `MeshInstance::bones` as `(first, count)`. A bind pose count that differs from the bone count is an
error.

**On the GPU.** A mesh with joints gets a second vertex buffer of `SkinVertex { joints: [u32; 4],
weights: [f32; 4] }`. Skinned meshes use their own pipelines (`vs_mesh_skinned`, opaque or see-through,
one- or two-sided; the fragment shader is shared) and the shadow pass has `vs_shadow_skinned`. The bone
matrices live in a storage buffer (the main pass keeps one list for all views, each view's bones after
the earlier ones; the shadow pass has its own). The instance data's `maps.w` holds the first bone plus
one, so 0 means a rigid mesh. `skin_matrix` blends the four bone matrices by weight. Skinned meshes are
never frustum culled, since they move away from their bounds, and skinned cut-outs cast solid shadows.

## glTF skins and animations

`models::load` also reads each skinned primitive's joints and weights (weights made to sum to 1) and puts
`skins` (`joints` as node indices, `inverseBind` as column-major matrices) and `animations` in the model
description. Channels carry the node, `path` (`translation`, `rotation` or `scale`), `interpolation`
(`linear`, `step` or `cubic`) and the keys; cubic splines keep their in-tangent, value and out-tangent
per key. Morph target weights are skipped.

In scripts, `Model` builds the animations as clips (`Model.animations`, wrap mode `loop`), with paths
from the copy's root made of node names (`Node <index>` for unnamed ones). Cubic channels become keys with
the file's tangents. `instantiate` makes a `SkinnedMeshRenderer` for each skinned part once all the
bones exist, with the skin's bind poses, and adds an `Animation` playing the first clip when the model
has any.

## Limits

- One animator layer: no masks, additive layers, 2D blend trees or sub-state machines.
- No humanoid retargeting, root motion or inverse kinematics.
- Skinning is four bones per vertex. No morph targets or blend shapes.
- Clips are evaluated in script, once per frame, not per fixed tick.

## Tests

`M5-ANI-01` to `-04` (curves, clips, `Animation`, `Animator`) are in
[js/engine/test/animation.test.ts](../js/engine/test/animation.test.ts); `M5-ANI-05` (`__iron_animate`
runs after every `onUpdate`) is in [iron_script/tests/components.rs](../crates/iron_script/tests/components.rs).
`M5-SKN-01` (glTF skins and animations) is a unit test in `models.rs` using `tests/fixtures/assets/worm.glb`
(made by `make_worm.py`); `M5-SKN-02` (skinned parts, clips from a model, mesh bone weights) is in
`js/engine/test/models.test.ts`. The golden `M5-RND-21` (a tube bent by two bones, with a bent shadow) is
in [iron_render/tests/world.rs](../crates/iron_render/tests/world.rs). `examples/animation` is the manual
check.
