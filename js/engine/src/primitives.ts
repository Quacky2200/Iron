// `GameObject.createPrimitive`, like Unity's: an object with a built-in mesh and the
// matching collider (a cube gets a BoxCollider, a sphere a SphereCollider, ...).

import { Vector3 } from "./math.ts";
import { BoxCollider, CapsuleCollider, MeshCollider, SphereCollider } from "./physics-components.ts";
import { MeshRenderer, type PrimitiveMesh } from "./rendering.ts";
import { GameObject, type GameObjectOptions, setPrimitiveFactory } from "./scene.ts";

/** Unity's `PrimitiveType`. */
export const PrimitiveType = {
  Cube: "cube",
  Sphere: "sphere",
  Capsule: "capsule",
  Cylinder: "cylinder",
  Plane: "plane",
  Quad: "quad",
} as const satisfies Record<string, PrimitiveMesh>;

export function createPrimitive(type: PrimitiveMesh, options: GameObjectOptions & { name?: string } = {}): GameObject {
  const name = options.name ?? type[0]!.toUpperCase() + type.slice(1);
  const go = new GameObject(name, options);
  go.addComponent(MeshRenderer, { mesh: type });
  switch (type) {
    case "cube":
      go.addComponent(BoxCollider);
      break;
    case "sphere":
      go.addComponent(SphereCollider);
      break;
    case "capsule":
      go.addComponent(CapsuleCollider);
      break;
    case "cylinder":
      // Unity gives cylinders a capsule; a 2 m tall, 0.5 m radius one fits.
      go.addComponent(CapsuleCollider);
      break;
    case "plane":
      // A thin box the size of the 10 m plane.
      go.addComponent(BoxCollider, { size: new Vector3(10, 0.01, 10) });
      break;
    case "quad":
      go.addComponent(MeshCollider, {
        points: [new Vector3(-0.5, -0.5, 0), new Vector3(0.5, -0.5, 0), new Vector3(0.5, 0.5, 0), new Vector3(-0.5, 0.5, 0)],
        indices: [0, 1, 2, 0, 2, 3],
      });
      break;
  }
  return go;
}

setPrimitiveFactory(createPrimitive);
