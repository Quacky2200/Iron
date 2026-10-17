// glTF models (`.glb` / `.gltf`), Unity-prefab style: `Model.load` reads one (the engine
// uploads its meshes and textures), and `instantiate` builds its node tree as
// GameObjects with MeshRenderers, optionally with mesh colliders.

import { Color, Quaternion, Vector2, Vector3, type QuaternionLike, type Vector3Like } from "./math.ts";
import { MeshCollider } from "./physics-components.ts";
import { Material, Mesh, MeshRenderer } from "./rendering.ts";
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
}

interface ModelDescription {
  nodes: ModelNode[];
  roots: number[];
  materials: ModelMaterial[];
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
  private description: ModelDescription | null = null;
  private waiting: ((m: Model) => void)[] = [];
  private pending: { options: InstantiateOptions; root: GameObject }[] = [];

  private constructor(src: string) {
    this.id = nextModel++;
    this.src = src;
    models.set(this.id, this);
    __iron_op("render.model", this.id, src);
  }

  /** Loads a `.glb` or `.gltf` from the game folder (once per path). */
  static load(src: string): Model {
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
    const make = (index: number, parent: GameObject) => {
      const n = d.nodes[index]!;
      const go = new GameObject(n.name || `Node ${index}`, {
        parent,
        position: new Vector3(...n.position),
        rotation: new Quaternion(...n.rotation),
        scale: new Vector3(...n.scale),
      });
      go._modelPart = true;
      // One renderer per part: on the node itself, or on child objects when there are several.
      n.primitives.forEach((p, i) => {
        const holder = n.primitives.length === 1 ? go : new GameObject(`${go.name} ${i}`, { parent: go });
        holder._modelPart = true;
        const mesh = Mesh._engine(p.mesh);
        const material = p.material === null ? new Material() : (this.materials[p.material] ?? new Material());
        holder.addComponent(MeshRenderer, { mesh, material });
        if (options.colliders === "mesh" || options.colliders === "convex") {
          holder.addComponent(MeshCollider, { sharedMesh: mesh, convex: options.colliders === "convex" });
        }
      });
      for (const c of n.children) make(c, go);
    };
    for (const r of d.roots) make(r, root);
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

globalThis.__iron_models = (loaded: unknown[]) => {
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
