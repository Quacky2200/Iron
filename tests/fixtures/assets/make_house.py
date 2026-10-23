# SPDX-License-Identifier: Apache-2.0

"""Writes house.glb, the glTF test model (M5-GLB-01), next to this script.

A textured box ("House", 1 m) with a two-part roof ("Roof": a red pyramid without
normals, and a glowing unlit ridge) and an empty "Lamp" node turned 45 degrees. Run
it again after changing it: `python tests/fixtures/assets/make_house.py`.
"""

import json
import struct
import zlib
from pathlib import Path


def png(width, height, rows):
    """A minimal RGBA PNG from rows of (r, g, b, a) tuples."""
    raw = b"".join(b"\x00" + bytes(c for px in row for c in px) for row in rows)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def cube():
    faces = [
        ((1, 0, 0), (0, 0, -1), (0, 1, 0)),
        ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
        ((0, 1, 0), (1, 0, 0), (0, 0, -1)),
        ((0, -1, 0), (1, 0, 0), (0, 0, 1)),
        ((0, 0, 1), (1, 0, 0), (0, 1, 0)),
        ((0, 0, -1), (-1, 0, 0), (0, 1, 0)),
    ]
    pos, nrm, uv, idx = [], [], [], []
    for n, u, v in faces:
        base = len(pos)
        for su, sv, tu, tv in [(-1, -1, 0, 1), (1, -1, 1, 1), (1, 1, 1, 0), (-1, 1, 0, 0)]:
            pos.append(tuple(n[k] * 0.5 + u[k] * su * 0.5 + v[k] * sv * 0.5 for k in range(3)))
            nrm.append(n)
            uv.append((tu, tv))
        idx += [base, base + 1, base + 2, base, base + 2, base + 3]
    return pos, nrm, uv, idx


def main():
    blob = bytearray()
    views, accessors = [], []

    def add(data, fmt, count, kind, comp, target, minmax=None):
        while len(blob) % 4:
            blob.append(0)
        offset = len(blob)
        for item in data:
            blob.extend(struct.pack(fmt, *(item if isinstance(item, tuple) else (item,))))
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(blob) - offset, **({"target": target} if target else {})})
        acc = {"bufferView": len(views) - 1, "componentType": comp, "count": count, "type": kind}
        if minmax:
            acc["min"], acc["max"] = minmax
        accessors.append(acc)
        return len(accessors) - 1

    def vec3s(points):
        return add(points, "<3f", len(points), "VEC3", 5126, 34962, [[min(p[k] for p in points) for k in range(3)], [max(p[k] for p in points) for k in range(3)]])

    # The box.
    pos, nrm, uv, idx = cube()
    box = {
        "attributes": {"POSITION": vec3s(pos), "NORMAL": add(nrm, "<3f", len(nrm), "VEC3", 5126, 34962), "TEXCOORD_0": add(uv, "<2f", len(uv), "VEC2", 5126, 34962)},
        "indices": add(idx, "<H", len(idx), "SCALAR", 5123, 34963),
        "material": 1,
    }
    # The roof: a pyramid (no normals: the engine makes them) and a ridge bar.
    roof_pos = [(-0.6, 0, -0.6), (0.6, 0, -0.6), (0.6, 0, 0.6), (-0.6, 0, 0.6), (0, 0.6, 0)]
    roof_idx = [0, 4, 1, 1, 4, 2, 2, 4, 3, 3, 4, 0]
    roof = {"attributes": {"POSITION": vec3s(roof_pos)}, "indices": add(roof_idx, "<I", len(roof_idx), "SCALAR", 5125, 34963), "material": 0}
    ridge_pos, ridge_nrm, ridge_uv, ridge_idx = cube()
    ridge_pos = [(x * 0.1, y * 0.1 + 0.62, z * 0.1) for x, y, z in ridge_pos]
    ridge = {
        "attributes": {"POSITION": vec3s(ridge_pos), "NORMAL": add(ridge_nrm, "<3f", len(ridge_nrm), "VEC3", 5126, 34962)},
        "indices": add(ridge_idx, "<H", len(ridge_idx), "SCALAR", 5123, 34963),
        "material": 2,
    }
    # A 4 x 4 checker image, embedded.
    light, dark = (235, 220, 180, 255), (150, 90, 50, 255)
    rows = [[light if (x + y) % 2 == 0 else dark for x in range(4)] for y in range(4)]
    image = png(4, 4, rows)
    while len(blob) % 4:
        blob.append(0)
    views.append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(image)})
    blob.extend(image)
    while len(blob) % 4:
        blob.append(0)

    doc = {
        "asset": {"version": "2.0", "generator": "Iron make_house.py"},
        "extensionsUsed": ["KHR_materials_unlit"],
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [
            {"name": "House", "mesh": 0, "children": [1, 2]},
            {"name": "Roof", "mesh": 1, "translation": [0, 0.5, 0]},
            {"name": "Lamp", "translation": [0.7, 0.2, 0.5], "rotation": [0, 0.3826834, 0, 0.9238795]},
        ],
        "meshes": [{"name": "Box", "primitives": [box]}, {"name": "Roof", "primitives": [roof, ridge]}],
        "materials": [
            {"name": "Red", "pbrMetallicRoughness": {"baseColorFactor": [0.8, 0.2, 0.1, 1], "roughnessFactor": 0.4}},
            {"name": "Checker", "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "roughnessFactor": 1}},
            {"name": "Glow", "emissiveFactor": [1, 0.6, 0.2], "pbrMetallicRoughness": {"baseColorFactor": [1, 0.8, 0.4, 1]}, "extensions": {"KHR_materials_unlit": {}}},
        ],
        "textures": [{"source": 0, "sampler": 0}],
        "samplers": [{"magFilter": 9728, "minFilter": 9728}],
        "images": [{"bufferView": len(views) - 1, "mimeType": "image/png"}],
        "buffers": [{"byteLength": len(blob)}],
        "bufferViews": views,
        "accessors": accessors,
    }
    text = json.dumps(doc, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    total = 12 + 8 + len(text) + 8 + len(blob)
    glb = struct.pack("<III", 0x46546C67, 2, total)
    glb += struct.pack("<II", len(text), 0x4E4F534A) + text
    glb += struct.pack("<II", len(blob), 0x004E4942) + bytes(blob)
    out = Path(__file__).with_name("house.glb")
    out.write_bytes(glb)
    print(f"wrote {out} ({len(glb)} bytes)")


main()
