// SPDX-License-Identifier: Apache-2.0

// Cloth (Unity's `Cloth`): put it on an object with a `MeshRenderer` (a "plane", "quad"
// or script `Mesh`) and the engine simulates a copy of the mesh on the fixed tick:
// gravity, wind, edges that keep their length, folds that resist, pinned vertices held
// where the object is, and sphere and capsule colliders it drapes over.

import { CapsuleCollider, SphereCollider } from "./physics-components.ts";
import { property } from "./property.ts";
import { Vector3, type Vector3Like } from "./math.ts";
import { Mesh, type MeshRenderer } from "./rendering.ts";
import { Component, type GameObject } from "./scene.ts";

/** A box in the object's space (`min` to `max` corners). */
export interface PinBox {
  min: Vector3Like;
  max: Vector3Like;
}

/**
 * Simulates its object's mesh as cloth. Settings take effect when it starts or when
 * `rebuild()` is called (the cloth then starts over from the mesh's shape).
 */
export class Cloth extends Component {
  /** 0 (rubbery) to 1 (keeps its edges' lengths). */
  @property stretchingStiffness = 1;
  /** 0 (folds freely) to 1 (resists folding). */
  @property bendingStiffness = 0.3;
  /** 0..1: how much of its motion it loses each second. */
  @property damping = 0.1;
  @property useGravity = true;
  /** A steady acceleration on every vertex (steady wind). */
  @property externalAcceleration: Vector3 = Vector3.zero;
  /** Up to this much more, varying (gusts). */
  @property randomAcceleration: Vector3 = Vector3.zero;
  /** 0..1: how much sliding over colliders slows it. */
  @property friction = 0.5;
  /** Constraint passes per step: more is stiffer, and slower to run. */
  @property solverIterations = 8;
  /** Vertices within `pinRadius` of these points (object space) stay put. */
  @property pins: Vector3[] = [];
  @property pinRadius = 0.05;
  /** Vertices in this object-space box stay put (e.g. a flag's edge on its pole). */
  @property pinBox: PinBox | null = null;
  /** Objects with a `SphereCollider` the cloth can't pass through. */
  @property sphereColliders: GameObject[] = [];
  /** Objects with a `CapsuleCollider` the cloth can't pass through. */
  @property capsuleColliders: GameObject[] = [];

  private renderer: MeshRenderer | undefined;
  private original: MeshRenderer["mesh"] | undefined;
  private simulated: Mesh | null = null;

  override onStart(): void {
    this.rebuild();
  }

  /** Starts the simulation again from the mesh, with the current settings. */
  rebuild(): void {
    const renderer = this.gameObject.components.find((c) => c.constructor.name === "MeshRenderer" || c.constructor.name === "SkinnedMeshRenderer") as
      | MeshRenderer
      | undefined;
    if (!renderer) {
      console.warn(`${this.gameObject.name}: Cloth needs a MeshRenderer on the same object`);
      return;
    }
    if (!this.simulated) {
      this.renderer = renderer;
      this.original = renderer.mesh;
      this.simulated = Mesh._reserve();
    }
    const source = this.original!;
    const v = (x: Vector3Like) => [x.x, x.y, x.z];
    const colliders: unknown[] = [];
    for (const go of this.sphereColliders) {
      const c = go.getComponent(SphereCollider);
      if (c) colliders.push(["sphere", go.id, c.radius, v(c.center)]);
    }
    for (const go of this.capsuleColliders) {
      const c = go.getComponent(CapsuleCollider);
      if (c) colliders.push(["capsule", go.id, c.radius, c.height, v(c.center)]);
    }
    const box = this.pinBox;
    __iron_op(
      "cloth.cloth",
      this.id,
      this.gameObject.id,
      typeof source === "string" ? source : source.id,
      this.simulated.id,
      [this.stretchingStiffness, this.bendingStiffness, this.damping, this.friction, this.solverIterations],
      v(this.externalAcceleration),
      v(this.randomAcceleration),
      this.useGravity,
      this.pins.flatMap(v),
      this.pinRadius,
      box ? [...v(box.min), ...v(box.max)] : null,
      colliders,
    );
    // The renderer draws the simulated copy.
    renderer.mesh = this.simulated;
  }

  override onDestroy(): void {
    __iron_op("cloth.remove", this.id);
    if (this.renderer && this.original !== undefined && !this.renderer.gameObject.destroyed) this.renderer.mesh = this.original;
    this.simulated?.destroy();
  }
}
