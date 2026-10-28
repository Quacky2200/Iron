# crowd

Two busy sidewalks: two city blocks 220 m apart, each a ring of pavement round a block of buildings with a road round
that, and on each a crowd of mannequins walking round in both directions. Every figure is a skinned model with its own
`Animator` (idle, walk and jog blended by speed), its own colours, height, pace and lane; figures slow down behind
others or step into a free lane, keep to their side of the pavement when they can, and now and then stop for a look in a
shop window. It is a test of animation culling (the animators' `cullingMode`) and frustum culling (what each view sees
of the two blocks), in the spirit of `fog-world`: the HUD counts what is drawn, seen, animated and culled.

Engine features exercised:

- `Model.load(..).whenLoaded()` and `model.instantiate` for hundreds of copies of one skinned model, made a few a frame;
  `SkinnedMeshRenderer` materials swapped per copy from a shared palette.
- One `AnimatorController` shared by every figure: a 1D blend tree by speed (`Idle_Loop`, `Walk_Loop`, `Jog_Fwd_Loop`,
  each with a `timeScale` matching its stride), `Animator.play(state, normalizedTime)` for a random phase,
  `setFloat("speed", ..)` every frame.
- `Animator.cullingMode` (`alwaysAnimate`, `cullUpdateTransforms`, `cullCompletely`) switched for every figure at once.
- `SkinnedMeshRenderer.isVisible` (Unity's `Renderer.isVisible`): whether the last frame saw the figure, in view or as a
  shadow caster whose shadow can fall in view.
- `RenderStats` (`meshes`, `culledMeshes`, `shadowCasters`, `shadowMaps`); frustum culling of skinned meshes by a box
  round their bones, in each view and in the sun's shadow maps.
- A directional sun with soft cascaded shadows, `QualitySettings.shadowDistance`, a procedural `skybox`, primitives
  (`MeshRenderer` cubes) for the street, a React HUD.

## Build and run

```bash
cd tools && cargo run -q -p platypus_build -- build ../examples/crowd
cd .. && cargo run --release -p platypus_app -- examples/crowd
```

Screenshot run: `platypus-app examples/crowd --screenshot out.png --frames 150` (the interactive window has crashed some
GPUs; the screenshot run is safest). A windowless benchmark of every view, count and mode:

```bash
target/release/platypus-app.exe examples/crowd --bench --bench-input tools/bench/inputs/crowd.json \
  --fixed-dt 0.0166667 --size 1280x720 --bench-out out.json
```

## Controls

| Input | Action |
|---|---|
| 1 | Street level on block A, from the road's edge looking along the south pavement (block B behind) |
| 2 | Bird's-eye over block A |
| 3 | Both blocks, far and high (nothing culled) |
| 4 | Bird's-eye over block B: block A is entirely off screen |
| 5 / 6 / 7 / 8 / 9 | 50 / 100 / 250 / 500 / 1000 figures per block (100 to start) |
| M | The animators' culling mode: AlwaysAnimate, CullUpdateTransforms, CullCompletely |

Views change with a short ease (0.8 s). There is no key to turn frustum culling off: the engine has no switch for it.

## Reading the HUD

- **fps, ms**: measured from the real clock over a quarter second (the built-in counter, top right, shows the same from
  the host's side; it is on in the window and off in screenshot runs unless `--fps`).
- **Figures**: the figures on the block. Changing the count adds them a few a frame (24) or removes them at once.
- **Seen**: figures whose body was seen in the last frame drawn (`isVisible`): in a camera's view, or a shadow caster
  in the sun's shadow maps whose shadow can fall in view. That test is conservative (a box swept along the light,
  hundreds of metres), which is why the sun here has no east-west part: with one, a block's shadow sweeps reach under
  the other block's view and dozens of its figures count as seen.
- **Animated**: figures whose pose was written this frame. Under AlwaysAnimate it is all of them; under the two culling
  modes it is the seen ones (the animators ask about the same last frame). CullUpdateTransforms still runs the state
  machines and clocks of the others; CullCompletely stops them.
- **Culled**: figures minus seen.
- **Meshes**: what the renderer drew and skipped over every view (`RenderStats.meshes`, `culledMeshes`): the street's
  boxes and two skinned meshes per figure (body and joints).
- **Shadows**: meshes drawn into the shadow maps and the number of maps (four sun cascades). Casters outside every
  cascade are left out.

What to look for: view 4 with CullCompletely against AlwaysAnimate (block A's animation drops out, its meshes are culled
and leave the shadow maps), and view 3 (everything in view: culling saves nothing, the modes cost the same).

## Structure

| File | What |
|---|---|
| `project.json` | `name`, `title` and `description`. |
| `tsconfig.json` | `tsc` options (`src/modules` for bare imports; there are none). |
| `src/main.tsx` | About 600 lines: the block's sizes and lanes, the ring path (`ring`), `buildBlock`, the `Crowd` component (spawning, thinking, moving, the camera, counting), the HUD. |
| `assets/mannequin.glb` | Quaternius's mannequin with eight of its animations (CC0, 1.9 MB, the same file as `fog-white-room`'s). |
| `assets/CREDITS.md` | Credits. |

## How it works

1. **The ring.** Each figure has a place round the ring (`u`, 0 to 1), a direction and a distance out from the
   buildings (`d`): `ring(d, s)` gives the point `s` metres round a rounded rectangle `d` metres out, and the way on.
   Eight lanes cross the pavement; anticlockwise walkers start in the inner four, clockwise ones in the outer four.
2. **Thinking.** Each figure decides every sixth frame (staggered): neighbours come from cells 2.5 m long round the
   ring, rebuilt each frame. Someone slower ahead in its lane, or someone coming the other way, sends it to a free
   lane beside (nearer its own side first), or else it slows to the leader's pace or stops; out of its side's lanes it
   drifts back when there is room. Now and then a walker stops for 2 to 6 s.
3. **Moving.** Speed eases (quicker to slow down), lane changes slide at 0.7 m/s, and the figure faces along its way,
   turned a little into a lane change. The animator's `speed` parameter is the figure's speed.
4. **Counting.** Every quarter second the HUD reads `isVisible` of each figure's body. Asking starts the engine watching
   that renderer (a test per view and shadow map per frame), which the default `alwaysAnimate` would not need: the HUD's
   own cost.

## Gotchas

- Far planes are short where they can be (200 m over a block, 700 m for the wide view), for the same reason: a deep
  frustum is easier for a shadow sweep to reach.
- Figures behind the buildings in views 2 and 4 count as seen: culling is by view and shadow, not occlusion.
- Figures walk through each other now and then: the avoidance is loose (no physics).
- The cost per figure is mostly the animation (two clips blended for about 65 bones) and the bone matrices sent to the
  GPU every frame for every figure, seen or not.

Related examples: fog-white-room, fog-world, animation, layers
