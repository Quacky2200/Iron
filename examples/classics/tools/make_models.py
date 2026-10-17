"""Converts the classic test models to .glb for examples/classics/assets.

    python examples/classics/tools/make_models.py <bun_zipper_res2.ply> <newell_teaset dir>

Sources (download them yourself; they're not in the repo):
- The Stanford Bunny: graphics.stanford.edu/pub/3Dscanrep/bunny.tar.gz,
  bunny/reconstruction/bun_zipper_res2.ply. Stanford Computer Graphics Laboratory.
- The Newell teaset (teapot, teacup, teaspoon): users.cs.utah.edu/~natevm/newell_teaset/.
  Martin Newell, University of Utah, 1975.

The bunny is scaled to 1 m tall; the teaset shares one scale (the teapot 1 m tall). All
stand on y = 0 (all are y-up already) and are centred in x and z.
Normals are smoothed for the bunny (its scan has none); UVs flip to glTF's top-down v.
"""

import json
import math
import struct
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "assets"


def write_glb(path, positions, normals, uvs, indices, material):
    blob = bytearray()
    views, accessors = [], []

    def add(data, fmt, kind, comp, target, minmax=None):
        while len(blob) % 4:
            blob.append(0)
        offset = len(blob)
        for item in data:
            blob.extend(struct.pack(fmt, *(item if isinstance(item, (tuple, list)) else (item,))))
        views.append({"buffer": 0, "byteOffset": offset, "byteLength": len(blob) - offset, "target": target})
        acc = {"bufferView": len(views) - 1, "componentType": comp, "count": len(data), "type": kind}
        if minmax:
            acc["min"], acc["max"] = minmax
        accessors.append(acc)
        return len(accessors) - 1

    lo = [min(p[k] for p in positions) for k in range(3)]
    hi = [max(p[k] for p in positions) for k in range(3)]
    attrs = {
        "POSITION": add(positions, "<3f", "VEC3", 5126, 34962, [lo, hi]),
        "NORMAL": add(normals, "<3f", "VEC3", 5126, 34962),
    }
    if uvs:
        attrs["TEXCOORD_0"] = add(uvs, "<2f", "VEC2", 5126, 34962)
    prim = {"attributes": attrs, "indices": add(indices, "<I", "SCALAR", 5125, 34963), "material": 0}
    while len(blob) % 4:
        blob.append(0)
    doc = {
        "asset": {"version": "2.0", "generator": "Iron make_models.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": path.stem, "mesh": 0}],
        "meshes": [{"name": path.stem, "primitives": [prim]}],
        "materials": [material],
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
    path.write_bytes(glb)
    print(f"wrote {path.name}: {len(positions)} vertices, {len(indices) // 3} triangles, {len(glb)} bytes")


def smooth_normals(positions, indices):
    acc = [[0.0, 0.0, 0.0] for _ in positions]
    for t in range(0, len(indices), 3):
        a, b, c = (positions[i] for i in indices[t : t + 3])
        u = [b[k] - a[k] for k in range(3)]
        v = [c[k] - a[k] for k in range(3)]
        # Area-weighted face normal.
        n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
        for i in indices[t : t + 3]:
            for k in range(3):
                acc[i][k] += n[k]
    out = []
    for n in acc:
        length = math.sqrt(sum(x * x for x in n)) or 1.0
        out.append(tuple(x / length for x in n))
    return out


def fit(positions, scale, offset):
    return [tuple((p[k] + offset[k]) * scale for k in range(3)) for p in positions]


def placement(positions, height):
    """Scale to `height` tall and an offset that stands it on y = 0, centred."""
    lo = [min(p[k] for p in positions) for k in range(3)]
    hi = [max(p[k] for p in positions) for k in range(3)]
    scale = height / (hi[1] - lo[1])
    offset = (-(lo[0] + hi[0]) / 2, -lo[1], -(lo[2] + hi[2]) / 2)
    return scale, offset


def read_ply(path):
    lines = Path(path).read_text().splitlines()
    end = lines.index("end_header")
    header = lines[:end]
    nv = int(next(l for l in header if l.startswith("element vertex")).split()[-1])
    nf = int(next(l for l in header if l.startswith("element face")).split()[-1])
    body = lines[end + 1 :]
    positions = [tuple(float(x) for x in body[i].split()[:3]) for i in range(nv)]
    indices = []
    for line in body[nv : nv + nf]:
        f = [int(x) for x in line.split()]
        poly = f[1 : 1 + f[0]]
        for i in range(1, len(poly) - 1):
            indices += [poly[0], poly[i], poly[i + 1]]
    return positions, indices


def read_obj(path, z_up):
    vs, vts, vns = [], [], []
    corners = {}
    positions, normals, uvs, indices = [], [], [], []
    # z-up to y-up: (x, y, z) -> (x, z, -y).
    turn = (lambda p: (p[0], p[2], -p[1])) if z_up else (lambda p: p)
    for line in Path(path).read_text().splitlines():
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "v":
            vs.append(turn(tuple(float(x) for x in parts[1:4])))
        elif parts[0] == "vt":
            vts.append((float(parts[1]), 1 - float(parts[2])))
        elif parts[0] == "vn":
            vns.append(turn(tuple(float(x) for x in parts[1:4])))
        elif parts[0] == "f":
            poly = []
            for corner in parts[1:]:
                if corner not in corners:
                    v, t, n = (corner.split("/") + ["", ""])[:3]
                    corners[corner] = len(positions)
                    positions.append(vs[int(v) - 1])
                    uvs.append(vts[int(t) - 1] if t else (0.0, 0.0))
                    normals.append(vns[int(n) - 1] if n else (0.0, 1.0, 0.0))
                poly.append(corners[corner])
            for i in range(1, len(poly) - 1):
                indices += [poly[0], poly[i], poly[i + 1]]
    return positions, normals, uvs, indices


def material(name, color, roughness, metallic=0.0):
    return {"name": name, "pbrMetallicRoughness": {"baseColorFactor": color, "roughnessFactor": roughness, "metallicFactor": metallic}}


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    ply, teaset = sys.argv[1], Path(sys.argv[2])
    OUT.mkdir(parents=True, exist_ok=True)

    positions, indices = read_ply(ply)
    scale, offset = placement(positions, 1.0)
    positions = fit(positions, scale, offset)
    write_glb(OUT / "bunny.glb", positions, smooth_normals(positions, indices), [], indices, material("Porcelain", [0.92, 0.9, 0.86, 1], 0.25))

    models = {name: read_obj(teaset / f"{name}.obj", z_up=False) for name in ("teapot", "teacup", "spoon")}
    # One scale for the set, from the teapot: they keep their sizes relative to each other.
    scale, _ = placement(models["teapot"][0], 1.0)
    colors = {
        "teapot": material("Glazed", [0.85, 0.32, 0.22, 1], 0.15),
        "teacup": material("China", [0.3, 0.5, 0.85, 1], 0.2),
        "spoon": material("Silver", [0.75, 0.76, 0.78, 1], 0.1, 1.0),
    }
    for name, (pos, nrm, uv, idx) in models.items():
        _, offset = placement(pos, 1.0)
        write_glb(OUT / f"{name}.glb", fit(pos, scale, offset), nrm, uv, idx, colors[name])


main()
