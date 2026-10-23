# SPDX-License-Identifier: Apache-2.0

"""Writes worm.glb, the skinned glTF test model (M5-SKN-01), next to this script.

A square tube 3 m tall ("Body") skinned to a chain of three joints ("Bone0" at the
bottom, "Bone1" 1 m up, "Bone2" 2 m up) under a "Worm" root, with an animation
"wave" (2 s): Bone1 and Bone2 bend 30 degrees about z and back (linear), and the root
steps 0.5 m along x at 1 s (step). Run it again after changing it:
`python tests/fixtures/assets/make_worm.py`.
"""

import json
import math
import struct
from pathlib import Path


def main():
    blob = bytearray()
    views, accessors = [], []

    def add(data, fmt, count, kind, comp, target=None, minmax=None):
        while len(blob) % 4:
            blob.append(0)
        offset = len(blob)
        for item in data:
            blob.extend(struct.pack(fmt, *(item if isinstance(item, tuple) else (item,))))
        view = {"buffer": 0, "byteOffset": offset, "byteLength": len(blob) - offset}
        if target:
            view["target"] = target
        views.append(view)
        acc = {"bufferView": len(views) - 1, "componentType": comp, "count": count, "type": kind}
        if minmax:
            acc["min"], acc["max"] = minmax
        accessors.append(acc)
        return len(accessors) - 1

    # The tube: rings of 4 corners every 0.5 m, outward normals.
    corners = [(-0.1, -0.1), (0.1, -0.1), (0.1, 0.1), (-0.1, 0.1)]
    pos, nrm, joints, weights, idx = [], [], [], [], []
    rings = 7
    for r in range(rings):
        y = r * 0.5
        # Weights blend between the joints below and above.
        if y <= 1:
            j, w = (0, 1, 0, 0), (1 - y, y, 0, 0)
        elif y <= 2:
            j, w = (1, 2, 0, 0), (2 - y, y - 1, 0, 0)
        else:
            j, w = (2, 0, 0, 0), (1, 0, 0, 0)
        for x, z in corners:
            pos.append((x, y, z))
            length = math.hypot(x, z)
            nrm.append((x / length, 0.0, z / length))
            joints.append(j)
            weights.append(w)
    for r in range(rings - 1):
        for k in range(4):
            a = r * 4 + k
            b = r * 4 + (k + 1) % 4
            c, d = a + 4, b + 4
            idx += [a, b, d, a, d, c]
    mn = [min(p[k] for p in pos) for k in range(3)]
    mx = [max(p[k] for p in pos) for k in range(3)]
    prim = {
        "attributes": {
            "POSITION": add(pos, "<3f", len(pos), "VEC3", 5126, 34962, [mn, mx]),
            "NORMAL": add(nrm, "<3f", len(nrm), "VEC3", 5126, 34962),
            "JOINTS_0": add(joints, "<4H", len(joints), "VEC4", 5123, 34962),
            "WEIGHTS_0": add(weights, "<4f", len(weights), "VEC4", 5126, 34962),
        },
        "indices": add(idx, "<H", len(idx), "SCALAR", 5123, 34963),
    }
    # Inverse bind matrices (column-major): the joints sit at y = 0, 1, 2.
    ibm = []
    for y in (0.0, 1.0, 2.0):
        ibm.append((1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, -y, 0, 1))
    ibm_acc = add(ibm, "<16f", 3, "MAT4", 5126)

    # The animation.
    times = [0.0, 1.0, 2.0]
    bend = math.radians(30) / 2
    rots = [(0, 0, 0, 1), (0, 0, math.sin(bend), math.cos(bend)), (0, 0, 0, 1)]
    t_acc = add(times, "<f", 3, "SCALAR", 5126, None, [[0.0], [2.0]])
    r_acc = add(rots, "<4f", 3, "VEC4", 5126)
    step_times = add([0.0, 1.0], "<f", 2, "SCALAR", 5126, None, [[0.0], [1.0]])
    step_values = add([(0, 0, 0), (0.5, 0, 0)], "<3f", 2, "VEC3", 5126)
    animation = {
        "name": "wave",
        "samplers": [
            {"input": t_acc, "output": r_acc, "interpolation": "LINEAR"},
            {"input": step_times, "output": step_values, "interpolation": "STEP"},
        ],
        "channels": [
            {"sampler": 0, "target": {"node": 2, "path": "rotation"}},
            {"sampler": 0, "target": {"node": 3, "path": "rotation"}},
            {"sampler": 1, "target": {"node": 0, "path": "translation"}},
        ],
    }

    doc = {
        "asset": {"version": "2.0", "generator": "make_worm.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [
            {"name": "Worm", "children": [1, 4]},
            {"name": "Bone0", "children": [2]},
            {"name": "Bone1", "translation": [0, 1, 0], "children": [3]},
            {"name": "Bone2", "translation": [0, 1, 0]},
            {"name": "Body", "mesh": 0, "skin": 0},
        ],
        "meshes": [{"name": "Body", "primitives": [prim]}],
        "skins": [{"joints": [1, 2, 3], "inverseBindMatrices": ibm_acc, "skeleton": 1}],
        "animations": [animation],
        "buffers": [{"byteLength": 0}],
        "bufferViews": views,
        "accessors": accessors,
    }
    while len(blob) % 4:
        blob.append(0)
    doc["buffers"][0]["byteLength"] = len(blob)
    text = json.dumps(doc, separators=(",", ":")).encode()
    while len(text) % 4:
        text += b" "
    out = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(text) + 8 + len(blob))
    out += struct.pack("<II", len(text), 0x4E4F534A) + text
    out += struct.pack("<II", len(blob), 0x004E4942) + bytes(blob)
    Path(__file__).with_name("worm.glb").write_bytes(out)


if __name__ == "__main__":
    main()
