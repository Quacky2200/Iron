# Copyright 2026 The Platypus Authors
# SPDX-License-Identifier: Apache-2.0

"""Writes the FBX test models (M5-FBX-*) next to this script, as ASCII FBX 7.4:

- cube.fbx: a 100 cm cube one metre up in a Z-up, centimetre file (so it comes in as a 1 m
  cube at y = 1 in metres, Y-up), with two materials (red on three faces, blue with the
  texture cube_tex.png on the other three), normals and UVs, and an empty "Pivot" child.
- rig.fbx: a 2 m column skinned to two bones ("Root", and "Tip" one metre up), the top
  vertices following Tip, and one take, "Bend", turning Tip 90 degrees about X over a second.

Run it again after changing it: `python tests/fixtures/assets/make_fbx.py`. Made here (no
downloads), so the files are the project's own (Apache-2.0, like the rest of the tests).
"""

import struct
import zlib
from pathlib import Path

HERE = Path(__file__).parent


def png(width, height, rows):
    """A minimal RGBA PNG from rows of (r, g, b, a) tuples."""
    raw = b"".join(b"\x00" + bytes(c for px in row for c in px) for row in rows)

    def chunk(kind, data):
        body = kind + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body))

    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")


def arr(kind, values):
    """An FBX array property: `Name: *N { a: v,v,v }`."""
    text = ",".join(fmt(v) for v in values)
    return f"{kind}: *{len(values)} {{\n\t\t\ta: {text}\n\t\t}}"


def fmt(v):
    if isinstance(v, float):
        return repr(v)
    return str(v)


HEADER = """; FBX 7.4.0 project file
; Written by tests/fixtures/assets/make_fbx.py
FBXHeaderExtension:  {
\tFBXHeaderVersion: 1003
\tFBXVersion: 7400
\tCreator: "Platypus test fixtures"
}
GlobalSettings:  {
\tVersion: 1000
\tProperties70:  {
\t\tP: "UpAxis", "int", "Integer", "",2
\t\tP: "UpAxisSign", "int", "Integer", "",1
\t\tP: "FrontAxis", "int", "Integer", "",1
\t\tP: "FrontAxisSign", "int", "Integer", "",-1
\t\tP: "CoordAxis", "int", "Integer", "",0
\t\tP: "CoordAxisSign", "int", "Integer", "",1
\t\tP: "UnitScaleFactor", "double", "Number", "",1
\t\tP: "TimeMode", "enum", "", "",11
\t}
}
"""


def box_faces(x0, x1, y0, y1, z0, z1):
    """The 8 corners and 6 quads (outward, counter-clockwise) of a box, Z up."""
    v = [
        (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
        (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1),
    ]
    quads = [
        (0, 3, 2, 1),  # bottom (-z)
        (4, 5, 6, 7),  # top (+z)
        (0, 1, 5, 4),  # front (-y)
        (1, 2, 6, 5),  # right (+x)
        (2, 3, 7, 6),  # back (+y)
        (3, 0, 4, 7),  # left (-x)
    ]
    normals = [(0, 0, -1), (0, 0, 1), (0, -1, 0), (1, 0, 0), (0, 1, 0), (-1, 0, 0)]
    return v, quads, normals


def mesh_block(gid, name, verts, quads, normals, materials):
    flat = [float(c) for p in verts for c in p]
    index = []
    for q in quads:
        index += [q[0], q[1], q[2], -q[3] - 1]
    norms = [float(c) for n in normals for _ in range(4) for c in n]
    uvs = [0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]
    uv_index = [0, 1, 2, 3] * len(quads)
    return f"""\tGeometry: {gid}, "Geometry::{name}", "Mesh" {{
\t\t{arr("Vertices", flat)}
\t\t{arr("PolygonVertexIndex", index)}
\t\tGeometryVersion: 124
\t\tLayerElementNormal: 0 {{
\t\t\tVersion: 102
\t\t\tName: ""
\t\t\tMappingInformationType: "ByPolygonVertex"
\t\t\tReferenceInformationType: "Direct"
\t\t\t{arr("Normals", norms)}
\t\t}}
\t\tLayerElementUV: 0 {{
\t\t\tVersion: 101
\t\t\tName: "UVMap"
\t\t\tMappingInformationType: "ByPolygonVertex"
\t\t\tReferenceInformationType: "IndexToDirect"
\t\t\t{arr("UV", uvs)}
\t\t\t{arr("UVIndex", uv_index)}
\t\t}}
\t\tLayerElementMaterial: 0 {{
\t\t\tVersion: 101
\t\t\tName: ""
\t\t\tMappingInformationType: "ByPolygon"
\t\t\tReferenceInformationType: "IndexToDirect"
\t\t\t{arr("Materials", materials)}
\t\t}}
\t\tLayer: 0 {{
\t\t\tVersion: 100
\t\t\tLayerElement:  {{
\t\t\t\tType: "LayerElementNormal"
\t\t\t\tTypedIndex: 0
\t\t\t}}
\t\t\tLayerElement:  {{
\t\t\t\tType: "LayerElementUV"
\t\t\t\tTypedIndex: 0
\t\t\t}}
\t\t\tLayerElement:  {{
\t\t\t\tType: "LayerElementMaterial"
\t\t\t\tTypedIndex: 0
\t\t\t}}
\t\t}}
\t}}
"""


def model_block(mid, name, kind, translation=(0, 0, 0), rotation=(0, 0, 0)):
    t = ",".join(fmt(float(c)) for c in translation)
    r = ",".join(fmt(float(c)) for c in rotation)
    return f"""\tModel: {mid}, "Model::{name}", "{kind}" {{
\t\tVersion: 232
\t\tProperties70:  {{
\t\t\tP: "Lcl Translation", "Lcl Translation", "", "A",{t}
\t\t\tP: "Lcl Rotation", "Lcl Rotation", "", "A",{r}
\t\t}}
\t\tShading: T
\t\tCulling: "CullingOff"
\t}}
"""


def material_block(mid, name, rgb):
    c = ",".join(fmt(float(x)) for x in rgb)
    return f"""\tMaterial: {mid}, "Material::{name}", "" {{
\t\tVersion: 102
\t\tShadingModel: "phong"
\t\tMultiLayer: 0
\t\tProperties70:  {{
\t\t\tP: "DiffuseColor", "Color", "", "A",{c}
\t\t\tP: "DiffuseFactor", "Number", "", "A",1
\t\t}}
\t}}
"""


def cube_fbx():
    verts, quads, normals = box_faces(-50, 50, -50, 50, -50, 50)
    objects = [
        mesh_block(1000, "Cube", verts, quads, normals, [0, 0, 0, 1, 1, 1]),
        model_block(2000, "Cube", "Mesh", translation=(0, 0, 100)),
        model_block(2001, "Pivot", "Null", translation=(0, 0, 50)),
        material_block(3000, "Red", (1, 0, 0)),
        material_block(3001, "Blue", (0.2, 0.4, 1)),
        """\tTexture: 4000, "Texture::Checker", "" {
\t\tType: "TextureVideoClip"
\t\tVersion: 202
\t\tTextureName: "Texture::Checker"
\t\tFileName: "cube_tex.png"
\t\tRelativeFilename: "cube_tex.png"
\t}
""",
    ]
    connections = [
        'C: "OO",2000,0',
        'C: "OO",1000,2000',
        'C: "OO",2001,2000',
        'C: "OO",3000,2000',
        'C: "OO",3001,2000',
        'C: "OP",4000,3001, "DiffuseColor"',
    ]
    return HEADER + "Objects:  {\n" + "".join(objects) + "}\nConnections:  {\n" + "".join(f"\t{c}\n" for c in connections) + "}\n"


def rig_fbx():
    # A column 20 x 20 x 200 cm: bottom ring at z = 0, middle at 100, top at 200, as two boxes
    # sharing nothing (simple to weight): the lower one follows Root, the upper one Tip.
    lower, quads, normals = box_faces(-10, 10, -10, 10, 0, 100)
    upper, _, _ = box_faces(-10, 10, -10, 10, 100, 200)
    verts = lower + upper
    all_quads = quads + [tuple(i + 8 for i in q) for q in quads]
    all_normals = normals + normals
    mesh = mesh_block(1000, "Column", verts, all_quads, all_normals, [0] * 12)
    ticks = 46186158000  # FBX ticks per second
    objects = [
        mesh,
        model_block(2000, "Column", "Mesh"),
        model_block(2001, "Root", "LimbNode"),
        model_block(2002, "Tip", "LimbNode", translation=(0, 0, 100)),
        material_block(3000, "Grey", (0.6, 0.6, 0.6)),
        """\tNodeAttribute: 2101, "NodeAttribute::Root", "LimbNode" {
\t\tTypeFlags: "Skeleton"
\t}
\tNodeAttribute: 2102, "NodeAttribute::Tip", "LimbNode" {
\t\tTypeFlags: "Skeleton"
\t}
\tDeformer: 5000, "Deformer::Skin", "Skin" {
\t\tVersion: 101
\t\tLink_DeformAcuracy: 50
\t}
""",
    ]
    # `TransformLink` is the bone's world matrix at bind time; `Transform` maps the mesh to the
    # bone (the inverse bind matrix), as exporters write it. Column-major, translation last.
    identity = [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0]
    tip_bind = [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 100.0, 1.0]
    tip_inverse = [1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, 0, 1.0, 0, 0, 0, -100.0, 1.0]
    clusters = [(5001, "Root", list(range(8)), identity, identity), (5002, "Tip", list(range(8, 16)), tip_inverse, tip_bind)]
    for did, name, idx, to_bone, link in clusters:
        objects.append(f"""\tDeformer: {did}, "SubDeformer::{name}", "Cluster" {{
\t\tVersion: 100
\t\tUserData: "", ""
\t\t{arr("Indexes", idx)}
\t\t{arr("Weights", [1.0] * len(idx))}
\t\t{arr("Transform", [float(x) for x in to_bone])}
\t\t{arr("TransformLink", [float(x) for x in link])}
\t}}
""")
    objects.append(f"""\tAnimationStack: 6000, "AnimStack::Bend", "" {{
\t\tProperties70:  {{
\t\t\tP: "LocalStop", "KTime", "Time", "",{ticks}
\t\t\tP: "ReferenceStop", "KTime", "Time", "",{ticks}
\t\t}}
\t}}
\tAnimationLayer: 6001, "AnimLayer::BaseLayer", "" {{
\t}}
\tAnimationCurveNode: 6002, "AnimCurveNode::R", "" {{
\t\tProperties70:  {{
\t\t\tP: "d|X", "Number", "", "A",0
\t\t\tP: "d|Y", "Number", "", "A",0
\t\t\tP: "d|Z", "Number", "", "A",0
\t\t}}
\t}}
\tAnimationCurve: 6003, "AnimCurve::", "" {{
\t\tDefault: 0
\t\tKeyVer: 4009
\t\t{arr("KeyTime", [0, ticks])}
\t\t{arr("KeyValueFloat", [0.0, 90.0])}
\t\t{arr("KeyAttrFlags", [8456])}
\t\t{arr("KeyAttrDataFloat", [0.0, 0.0, 0.0, 0.0])}
\t\t{arr("KeyAttrRefCount", [2])}
\t}}
""")
    connections = [
        'C: "OO",2000,0',
        'C: "OO",2001,0',
        'C: "OO",2002,2001',
        'C: "OO",2101,2001',
        'C: "OO",2102,2002',
        'C: "OO",1000,2000',
        'C: "OO",3000,2000',
        'C: "OO",5000,1000',
        'C: "OO",5001,5000',
        'C: "OO",5002,5000',
        'C: "OO",2001,5001',
        'C: "OO",2002,5002',
        'C: "OO",6001,6000',
        'C: "OO",6002,6001',
        'C: "OP",6002,2002, "Lcl Rotation"',
        'C: "OP",6003,6002, "d|X"',
    ]
    return HEADER + "Objects:  {\n" + "".join(objects) + "}\nConnections:  {\n" + "".join(f"\t{c}\n" for c in connections) + "}\n"


def main():
    (HERE / "cube.fbx").write_text(cube_fbx(), newline="\n")
    (HERE / "rig.fbx").write_text(rig_fbx(), newline="\n")
    checker = [[(255, 255, 255, 255) if (x + y) % 2 == 0 else (40, 40, 40, 255) for x in range(4)] for y in range(4)]
    (HERE / "cube_tex.png").write_bytes(png(4, 4, checker))


if __name__ == "__main__":
    main()
