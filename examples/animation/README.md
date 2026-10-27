# animation

Three kinds of animation in one scene. (1) A robot made of cube and sphere primitives, driven by an
`Animator` with a locomotion blend tree (idle, walk, run chosen by a `speed` parameter) and a jump state
entered by a trigger and left by exit time. (2) A worm loaded from a skinned glTF file that plays its own
animation. (3) A tentacle whose skinned mesh, bone weights and bind poses are built in code and bent by a looping
`AnimationClip` on its bone chain.

APIs used: `AnimationCurve.fromPoints`, `AnimationClip.setCurve`, `AnimatorController` (`addParameter`,
`addState`, `addTransition`), `Animator` (`setFloat`, `setTrigger`, `getCurrentAnimatorStateInfo`), `Animation`,
`SkinnedMeshRenderer`, `Mesh.create` with `boneWeights` and `bindposes`, `Matrix4x4.trs`, `Model.load`, `CameraOrbit`.

## Build and run

```bash
cd tools && cargo run -q -p iron_build -- build ../examples/animation
```

Then, from the repo root (`cd ..`):

```bash
cargo run -p iron_app -- examples/animation
```

Screenshot run: `cargo run -p iron_app -- examples/animation --screenshot out.png --frames 60`. The
interactive window may crash some GPUs; the screenshot run is safest. Package with
`iron-build package ../examples/animation`.

## Structure

| File | What |
|---|---|
| `project.json` | `name` and `title` only |
| `tsconfig.json` | Base config plus `"jsx": "react-jsx"` |
| `src/main.tsx` | Clip builders (`wave`, `gait`, `jumpClip`), `limb`, `Driver`, `Stage`, `tentacle`, the React `Hud` |
| `assets/worm.glb` (+ `.meta`) | The skinned, animated worm |

## How it works

1. **Curves and clips.** `wave()` samples a sine into 17 points and makes a smooth looping curve. `gait()` makes
   a clip whose curves are addressed by hierarchy path, component type and property:

```ts
clip.setCurve("Body/Left leg", "Transform", "localEulerAngles.x", wave(legs, period));
clip.setCurve("Body/Right leg", "Transform", "localEulerAngles.x", wave(legs, period, Math.PI));
```

   Every gait lasts one period (1 s) so blending lines the steps up. `jumpClip()` does not set `wrapMode = "loop"` and
   moves the body up and raises the arms.
2. **Limbs.** `limb()` makes a pivot object at the top of the limb with a scaled cube child hanging down, so
   rotating the pivot swings the limb.
3. **The controller.** `Stage.onStart` builds it with a fluent API: a `speed` float and a `jump` trigger, a
   `Locomotion` state that blends three clips by `speed` (thresholds 0, 1, 2), a `Jump` state, and two
   transitions (trigger to jump, then back by `hasExitTime` at 0.95):

```ts
.addTransition({ from: "Locomotion", to: "Jump", conditions: [{ parameter: "jump", mode: "if" }], duration: 0.1 })
.addTransition({ from: "Jump", to: "Locomotion", hasExitTime: true, exitTime: 0.95, duration: 0.15 });
```

   It is attached with `robot.addComponent(Animator, { controller })`.
4. **Driver.** `Driver` is a `Component` registered with `{ spawn: false }` (it is added in code, not
   spawned at start). Each frame it either cycles `speed` automatically (`1 - cos` over 12 s, so 0 -> 2 -> 0,
   with a jump every 5 s) or, after the first arrow key or Space, follows input. Then
   `animator.setFloat("speed", ...)` is called and the current state name is read for the HUD.
5. **Worm.** One line: `Model.load("assets/worm.glb").instantiate({ position: ... })`. The model brings its own
   `Animation` component and skinned mesh.
6. **Tentacle.** `tentacle()` builds four nested bone objects, then a tube of 17 rings of 8 vertices with a tapering
   radius; each ring blends two neighbouring bones by weight (`boneWeights` with `indices` and `weights`
   arrays of four). Bind poses are the inverse of each bone's rest world matrix. A `SkinnedMeshRenderer`
   takes `mesh`, `material`, `bones` and `rootBone`. An `Animation` component plays the `sway` clip, which
   rotates bones 1-3 about z and x with phase offsets.
7. **HUD.** A React overlay shows the state name and speed. A module-level `refreshHud` function is set
   in a `useEffect` and called by `Driver` about ten times a second, because component state is outside React.

## Controls

| Key | Effect |
|---|---|
| Up / Down arrows | Raise or lower the robot's speed (0 to 2); takes over from the automatic cycle |
| Space | Jump trigger |

Without input the robot cycles through idle, walk and run by itself and jumps every few seconds.

## Things to try

- Add a clip (a wave, a crouch) as another state and a trigger for it.
- Change blend thresholds or add a fourth child to the blend tree.
- Make the tentacle longer by raising the bone and ring counts (remember the weights and bind poses).
- Drive `speed` from a Rigidbody's velocity or a character controller for a real walker (see `physics`, `platformer`).
- Load your own skinned glTF as the worm does and play its clips with the `Animation` component.

## Gotchas

- Animation curve paths are relative to the object holding the `Animator`/`Animation` and use object names
  (`Body/Left leg`), so names must be unique among siblings.
- Bind poses must match the rest pose of the bones when the mesh was modelled; wrong ones twist the mesh.
- `Driver` is registered with `spawn: false` or it would also spawn a stray instance.
- The source passes `{ speed: 1, speedParameter: undefined }` as the `Locomotion` state's options; I did not
  find what the explicit `undefined` is for (probably it keeps the shape of the options object explicit for the compiler).

Related examples: cloth, classics, platformer
