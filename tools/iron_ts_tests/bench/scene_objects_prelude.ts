// SPDX-License-Identifier: Apache-2.0

// The original GameObject over a scene with no engine behind it, with the native API's names:
// child count and indexer, a "no object" instead of null / undefined, `lookAtUp`, root listing.
import "./hypot_patch.ts";
import { GameObject, Transform, _resetSceneForTests } from "../../../js/engine/src/scene.ts";

(globalThis as any).__iron_op = () => {};
_resetSceneForTests();

const NONE = { destroyed: true, name: "", childCount: 0 };
const g = GameObject.prototype as any;
Object.defineProperty(g, "childCount", { get() { return this.children.length; } });
g.childAt = function (i: number) { return this.children[i] ?? NONE; };
const findChild = g.findChild;
g.findChild = function (name: string) { return findChild.call(this, name) ?? NONE; };
const setParent = g.setParent;
g.setParent = function (p: any, keepWorld = true) { return setParent.call(this, p === NONE ? null : p, keepWorld); };
const gf = GameObject as any;
gf.rootCount = () => GameObject.roots.length;
gf.rootAt = (i: number) => GameObject.roots[i] ?? NONE;
const byId = GameObject.byId;
gf.byId = (id: number) => byId(id) ?? NONE;
const find = GameObject.find;
gf.find = (name: string) => find(name) ?? NONE;
(Transform.prototype as any).lookAtUp = function (t: any, up: any) { return this.lookAt(t, up); };
(globalThis as any).__bench_reset = () => _resetSceneForTests();
