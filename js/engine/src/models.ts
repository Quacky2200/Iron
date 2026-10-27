// SPDX-License-Identifier: Apache-2.0

// glTF models (`.glb` / `.gltf`), Unity-prefab style: `Model.load` reads one (the engine
// uploads its meshes and textures), and `instantiate` builds its node tree as
// GameObjects with MeshRenderers, optionally with mesh colliders.

import { Animation, AnimationClip, AnimationCurve } from "./animation.ts";
import { Assets } from "./assets.ts";
import { Color, Matrix4x4, Quaternion, Vector2, Vector3, type QuaternionLike, type Vector3Like } from "./math.ts";
import { MeshCollider } from "./physics-components.ts";
import { Material, Mesh, MeshRenderer, SkinnedMeshRenderer } from "./rendering.ts";
import { GameObject } from "./scene.ts";
import { Texture } from "./textures.ts";

interface ModelMaterial {
  name: string;
  color: [number, number, number, number];
  emission: [number, number, number];
  smoothness: number;
  metallic: number;
  unlit: boolean;
  /** [id, width, height] of the base colour texture. */
  texture: [number, number, number] | null;
  alpha: "opaque" | "mask" | "blend";
  alphaCutoff: number;
  doubleSided: boolean;
  /** [texture id, scale]. */
  normal: [number, number] | null;
  /** Roughness in green, metallic in blue. */
  metallicRoughness: number | null;
  /** [texture id, strength], occlusion in red. */
  occlusion: [number, number] | null;
  emissive: number | null;
}

interface ModelNode {
  name: string;
  position: [number, number, number];
  rotation: [number, number, number, number];
  scale: [number, number, number];
  children: number[];
  primitives: { mesh: number; material: number | null; bounds: [[number, number, number], [number, number, number]] }[];
  /** Its skin (`skins` index), for skinned meshes. */
  skin?: number | null;
}

interface ModelSkin {
  /** Joint nodes, in the order the mesh's bone indices use. */
  joints: number[];
  /** Inverse bind matrices, 16 numbers each, column-major. */
  inverseBind: number[];
}

interface ModelAnimation {
  name: string;
  channels: {
    node: number;
    path: "translation" | "rotation" | "scale";
    interpolation: "linear" | "step" | "cubic";
    times: number[];
    /** Per key: 3 or 4 numbers (cubic: in-tangent, value, out-tangent). */
    values: number[];
  }[];
}

interface ModelDescription {
  nodes: ModelNode[];
  roots: number[];
  materials: ModelMaterial[];
  skins?: ModelSkin[];
  animations?: ModelAnimation[];
}

export interface InstantiateOptions {
  position?: Vector3Like;
  rotation?: QuaternionLike;
  scale?: Vector3Like;
  parent?: GameObject;
  /**
   * Colliders on every mesh: `mesh` (exact triangles, for static scenery) or `convex`
   * (hulls, which can move with a Rigidbody). Default none.
   */
  colliders?: "none" | "mesh" | "convex";
}

let nextModel = 1;
const models = new Map<number, Model>();
const cache = new Map<string, Model>();

/** A loaded glTF model, ready to place any number of copies of. */
export class Model {
  readonly id: number;
  readonly src: string;
  isLoaded = false;
  /** True when loading failed (the engine logs why). */
  failed = false;
  /** Its materials, as `Material`s (after loading). */
  materials: Material[] = [];
  /** Its meshes (after loading), one per glTF primitive. */
  meshes: Mesh[] = [];
  /**
   * Its animations as clips (after loading), with paths from the copy's root object.
   * Copies get an `Animation` playing the first one.
   */
  animations: AnimationClip[] = [];
  private description: ModelDescription | null = null;
  private waiting: ((m: Model) => void)[] = [];
  private pending: { options: InstantiateOptions; root: GameObject }[] = [];

  private constructor(src: string) {
    this.id = nextModel++;
    this.src = src;
    models.set(this.id, this);
    __platypus_op("render.model", this.id, src);
  }

  /** Loads a `.glb` or `.gltf` from the game folder, or `"guid:<guid>"` (once per file). */
  static load(source: string): Model {
    const src = Assets.resolve(source);
    let m = cache.get(src);
    if (!m) {
      m = new Model(src);
      cache.set(src, m);
    }
    return m;
  }

  /** Resolves once it has loaded (at once if it has). Rejects if it failed. */
  whenLoaded(): Promise<Model> {
    if (this.isLoaded) return Promise.resolve(this);
    if (this.failed) return Promise.reject(new Error(`model ${this.src} failed to load`));
    return new Promise((resolve, reject) => {
      this.waiting.push((m) => (m.failed ? reject(new Error(`model ${m.src} failed to load`)) : resolve(m)));
    });
  }

  /**
   * Places a copy: a GameObject named after the file, holding the model's nodes as
   * children. Works before loading finishes too: the nodes appear once it has.
   */
  instantiate(options: InstantiateOptions = {}): GameObject {
    const name = this.src.split("/").pop()?.replace(/\.(glb|gltf)$/i, "") ?? "Model";
    const root = new GameObject(name, {
      position: options.position,
      rotation: options.rotation,
      scale: options.scale,
      parent: options.parent,
    });
    this._buildInto(root, options);
    return root;
  }

  /** @internal Builds the model's nodes under `root` (once loaded), marking it as a copy for saving. */
  _buildInto(root: GameObject, options: InstantiateOptions = {}): void {
    root._model = options.colliders && options.colliders !== "none" ? { src: this.src, colliders: options.colliders } : { src: this.src };
    if (this.description) this.build(root, options);
    else if (!this.failed) this.pending.push({ options, root });
  }

  private build(root: GameObject, options: InstantiateOptions): void {
    const d = this.description!;
    const objects = new Map<number, GameObject>();
    const skinned: { holder: GameObject; mesh: Mesh; material: Material; skin: ModelSkin }[] = [];
    const make = (index: number, parent: GameObject) => {
      const n = d.nodes[index]!;
      const go = new GameObject(n.name || `Node ${index}`, {
        parent,
        position: new Vector3(...n.position),
        rotation: new Quaternion(...n.rotation),
        scale: new Vector3(...n.scale),
      });
      go._modelPart = true;
      objects.set(index, go);
      // One renderer per part: on the node itself, or on child objects when there are several.
      n.primitives.forEach((p, i) => {
        const holder = n.primitives.length === 1 ? go : new GameObject(`${go.name} ${i}`, { parent: go });
        holder._modelPart = true;
        const mesh = Mesh._engine(p.mesh);
        const material = p.material === null ? new Material() : (this.materials[p.material] ?? new Material());
        const skin = n.skin == null ? undefined : d.skins?.[n.skin];
        // Skinned parts wait for every bone to exist.
        if (skin) skinned.push({ holder, mesh, material, skin });
        else holder.addComponent(MeshRenderer, { mesh, material });
        if (options.colliders === "mesh" || options.colliders === "convex") {
          holder.addComponent(MeshCollider, { sharedMesh: mesh, convex: options.colliders === "convex" });
        }
      });
      for (const c of n.children) make(c, go);
    };
    for (const r of d.roots) make(r, root);
    for (const s of skinned) {
      s.mesh.bindposes = Array.from({ length: s.skin.joints.length }, (_, i) => new Matrix4x4(s.skin.inverseBind.slice(i * 16, i * 16 + 16)));
      const bones = s.skin.joints.map((j) => objects.get(j)).filter((b): b is GameObject => b !== undefined);
      s.holder.addComponent(SkinnedMeshRenderer, { mesh: s.mesh, material: s.material, bones, rootBone: bones[0] ?? null });
    }
    if (this.animations.length && !root.getComponent(Animation)) {
      root.addComponent(Animation, { clip: this.animations[0]!, clips: [...this.animations] });
    }
  }

  /** The glTF's animations as clips, paths by node names from the copy's root. */
  private clips(d: ModelDescription): AnimationClip[] {
    const paths = new Map<number, string>();
    const walk = (index: number, prefix: string) => {
      const n = d.nodes[index]!;
      const path = prefix + (n.name || `Node ${index}`);
      paths.set(index, path);
      for (const c of n.children) walk(c, `${path}/`);
    };
    for (const r of d.roots) walk(r, "");
    return (d.animations ?? []).map((a, i) => {
      const clip = new AnimationClip(a.name || `Animation ${i}`);
      clip.wrapMode = "loop";
      for (const ch of a.channels) {
        const path = paths.get(ch.node);
        if (path === undefined) continue;
        const [property, size] =
          ch.path === "translation" ? ["localPosition", 3] : ch.path === "rotation" ? ["localRotation", 4] : ["localScale", 3];
        const values = ch.path === "rotation" ? continuous(ch.values, ch.interpolation === "cubic") : ch.values;
        for (let k = 0; k < size; k++) {
          const curve =
            ch.interpolation === "cubic"
              ? cubic(ch.times, values, size, k)
              : AnimationCurve.fromPoints(
                  ch.times.map((t, i) => [t, values[i * size + k]!] as const),
                  ch.interpolation === "step" ? "step" : "linear",
                );
          clip.setCurve(path, "Transform", `${property}.${"xyzw"[k]}`, curve);
        }
      }
      return clip;
    });
  }

  /** @internal The engine's answer. */
  _loaded(description: ModelDescription | null): void {
    if (description) {
      this.description = description;
      this.isLoaded = true;
      this.materials = description.materials.map((m) => {
        const [r, g, b, a] = m.color;
        const texture = m.texture ? Texture._engine(...m.texture) : null;
        // The maps' sizes aren't needed (only the base texture's is asked for).
        const map = (id: number | null | undefined) => (id == null ? null : Texture._engine(id, 0, 0));
        return new Material(
          new Color(r, g, b, a),
          new Color(m.emission[0], m.emission[1], m.emission[2], 1),
          m.smoothness,
          m.unlit,
          texture,
          Vector2.one,
          Vector2.zero,
          m.metallic ?? 0,
        ).with({
          renderingMode: m.alpha === "mask" ? "cutout" : m.alpha === "blend" ? "fade" : "opaque",
          alphaCutoff: m.alphaCutoff ?? 0.5,
          doubleSided: m.doubleSided ?? false,
          normalMap: map(m.normal?.[0]),
          normalScale: m.normal?.[1] ?? 1,
          metallicGlossMap: map(m.metallicRoughness),
          occlusionMap: map(m.occlusion?.[0]),
          occlusionStrength: m.occlusion?.[1] ?? 1,
          emissionMap: map(m.emissive),
          gltfPacking: true,
        });
      });
      this.meshes = description.nodes.flatMap((n) => n.primitives.map((p) => Mesh._engine(p.mesh)));
      this.animations = this.clips(description);
      for (const { root, options } of this.pending) if (!root.destroyed) this.build(root, options);
    } else {
      this.failed = true;
    }
    this.pending = [];
    const waiting = this.waiting;
    this.waiting = [];
    for (const f of waiting) f(this);
  }
}

globalThis.__platypus_models = (loaded: unknown[]) => {
  for (let i = 0; i + 1 < loaded.length; i += 2) {
    const m = models.get(loaded[i] as number);
    const json = loaded[i + 1];
    m?._loaded(typeof json === "string" ? (JSON.parse(json) as ModelDescription) : null);
  }
};

/** Forgets every model. Only for tests. */
export function _resetModelsForTests() {
  models.clear();
  cache.clear();
  nextModel = 1;
}

/** A glTF cubic spline channel's component `k`: keys with the file's tangents. */
/**
 * Rotation keys made sign-continuous: q and -q are the same turn, and exporters (Blender's)
 * sometimes switch sides from one key to the next. Blended component by component, a key
 * pair on opposite sides passes near zero between them, and normalized that's a wild pose
 * for a frame or two (a leg flung up mid-stride). Each key is flipped to its predecessor's
 * side; cubic keys (in-tangent, value, out-tangent) flip as a whole.
 */
function continuous(values: number[], cubicKeys: boolean): number[] {
  const out = [...values];
  const stride = cubicKeys ? 12 : 4;
  const at = cubicKeys ? 4 : 0;
  for (let i = stride; i + stride <= out.length; i += stride) {
    let dot = 0;
    for (let c = 0; c < 4; c++) dot += out[i - stride + at + c]! * out[i + at + c]!;
    if (dot < 0) for (let c = 0; c < stride; c++) out[i + c] = -out[i + c]!;
  }
  return out;
}

function cubic(times: number[], values: number[], size: number, k: number): AnimationCurve {
  const keys = times.map((t, i) => {
    const at = (part: number) => values[(i * 3 + part) * size + k]!;
    return { t, inT: at(0), v: at(1), outT: at(2) };
  });
  const curve = new AnimationCurve();
  for (const key of keys) curve.addKey(key.t, key.v);
  curve.keys.forEach((key, i) => {
    key.inTangent = keys[i]!.inT;
    key.outTangent = keys[i]!.outT;
  });
  return curve;
}
