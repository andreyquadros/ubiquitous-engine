#!/usr/bin/env python3
"""
Rig, animate and optimise the UBI mascot from a raw Blender export, headless and deterministic.

    python rig_ubi.py <raw.glb> <out.glb> [--renders DIR] [--landmarks FILE] [--no-optimize] [--keep-temp]

Runs inside the `bpy` Python module (pip install bpy==5.0.1). Steps:

1. import the raw GLB and apply the object transform (Blender is Z-up: glTF +Y -> +Z, glTF +Z (front) -> -Y);
2. find the joints from the geometry with numpy (slices along the height, limb clusters, principal axes);
3. build the armature of the contract (Root > Hips > Spine > Chest > Neck > Head, arms, legs, Orb),
   rest pose = the pose of the export;
4. skin: rigid-with-blend by nearest bone segment inside body regions (see `compute_weights`);
5. author the eight clips (Idle, Yes, No, Wave, Jump, Excited, Worried, Sleep) as actions pushed to NLA tracks;
6. export a GLB, drop animation channels that only repeat the rest pose, then optimise with gltf-transform
   (textures 1024 px, dedup/prune, Draco) and verify the result (21 joints, 8 clips, size);
7. optionally render validation frames with Cycles (CPU) into --renders.

Everything the geometry analysis decides is printed and written to <out>.landmarks.json; pass an edited copy with
--landmarks to override any joint (only the keys you list are replaced).
"""

import argparse
import json
import math
import os
import shutil
import struct
import subprocess
import sys
import time

import numpy as np

import bpy
from mathutils import Matrix, Quaternion, Vector

# ----------------------------------------------------------------------------------------------------------------------
# Contract
# ----------------------------------------------------------------------------------------------------------------------

FPS = 24
PARENT = {
    'Root': None,
    'Hips': 'Root', 'Spine': 'Hips', 'Chest': 'Spine', 'Neck': 'Chest', 'Head': 'Neck',
    'Shoulder.L': 'Chest', 'UpperArm.L': 'Shoulder.L', 'LowerArm.L': 'UpperArm.L', 'Hand.L': 'LowerArm.L',
    'Shoulder.R': 'Chest', 'UpperArm.R': 'Shoulder.R', 'LowerArm.R': 'UpperArm.R', 'Hand.R': 'LowerArm.R',
    'UpperLeg.L': 'Hips', 'LowerLeg.L': 'UpperLeg.L', 'Foot.L': 'LowerLeg.L',
    'UpperLeg.R': 'Hips', 'LowerLeg.R': 'UpperLeg.R', 'Foot.R': 'LowerLeg.R',
    'Orb': 'Root',
}
BONES = list(PARENT)
CLIPS = ['Idle', 'Yes', 'No', 'Wave', 'Jump', 'Excited', 'Worried', 'Sleep']
# joint name -> (bone whose head it is). The bone table below maps bones to (head joint, tail joint).
BONE_JOINTS = {
    'Root': ('root', 'root_tip'),
    'Hips': ('pelvis', 'spine'), 'Spine': ('spine', 'chest'), 'Chest': ('chest', 'neck_base'),
    'Neck': ('neck_base', 'neck'), 'Head': ('neck', 'head_top'),
    'Shoulder.L': ('clavicle_L', 'shoulder_L'), 'UpperArm.L': ('shoulder_L', 'elbow_L'),
    'LowerArm.L': ('elbow_L', 'wrist_L'), 'Hand.L': ('wrist_L', 'fingertip_L'),
    'Shoulder.R': ('clavicle_R', 'shoulder_R'), 'UpperArm.R': ('shoulder_R', 'elbow_R'),
    'LowerArm.R': ('elbow_R', 'wrist_R'), 'Hand.R': ('wrist_R', 'fingertip_R'),
    'UpperLeg.L': ('hip_L', 'knee_L'), 'LowerLeg.L': ('knee_L', 'ankle_L'), 'Foot.L': ('ankle_L', 'toe_L'),
    'UpperLeg.R': ('hip_R', 'knee_R'), 'LowerLeg.R': ('knee_R', 'ankle_R'), 'Foot.R': ('ankle_R', 'toe_R'),
    'Orb': ('orb', 'orb_top'),
}
# blend radius (world units) around the joint at the head of each bone, between it and its parent
BLEND_RADIUS = {
    'Hips': 0.0, 'Spine': 0.16, 'Chest': 0.16, 'Neck': 0.12, 'Head': 0.12,
    'Shoulder.L': 0.10, 'UpperArm.L': 0.10, 'LowerArm.L': 0.09, 'Hand.L': 0.08,
    'Shoulder.R': 0.10, 'UpperArm.R': 0.10, 'LowerArm.R': 0.09, 'Hand.R': 0.08,
    'UpperLeg.L': 0.10, 'LowerLeg.L': 0.09, 'Foot.L': 0.08,
    'UpperLeg.R': 0.10, 'LowerLeg.R': 0.09, 'Foot.R': 0.08,
    'Orb': 0.0, 'Root': 0.0,
}


def log(*a):
    print('[rig_ubi]', *a, flush=True)


# ----------------------------------------------------------------------------------------------------------------------
# 1. import
# ----------------------------------------------------------------------------------------------------------------------

def import_raw(path):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    sc.render.fps = FPS
    bpy.ops.import_scene.gltf(filepath=path)
    meshes = [o for o in bpy.data.objects if o.type == 'MESH']
    if not meshes:
        raise SystemExit('no mesh in ' + path)
    if len(meshes) > 1:
        # one skinned mesh is simpler for the app: join everything
        for o in bpy.data.objects:
            o.select_set(o in meshes)
        bpy.context.view_layer.objects.active = meshes[0]
        bpy.ops.object.join()
    mesh = bpy.context.view_layer.objects.active if len(meshes) > 1 else meshes[0]
    # flatten: world transform into the vertices, drop any empties from the export
    for o in bpy.data.objects:
        o.select_set(o == mesh)
    bpy.context.view_layer.objects.active = mesh
    mesh.parent = None
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    for o in list(bpy.data.objects):
        if o != mesh:
            bpy.data.objects.remove(o, do_unlink=True)
    mesh.name = 'Ubi'
    mesh.data.name = 'Ubi'
    return mesh


def mesh_arrays(mesh):
    me = mesh.data
    n = len(me.vertices)
    co = np.empty(n * 3, dtype=np.float64)
    me.vertices.foreach_get('co', co)
    co = co.reshape(-1, 3)
    ne = len(me.edges)
    ev = np.empty(ne * 2, dtype=np.int64)
    me.edges.foreach_get('vertices', ev)
    return co, ev.reshape(-1, 2)


def islands(co, ev):
    """Connected components after merging vertices by position (glTF splits vertices on UV/normal seams)."""
    q = np.round(co, 5)
    _, uid = np.unique(q, axis=0, return_inverse=True)
    uid = uid.ravel()
    e = uid[ev]
    lab = np.arange(uid.max() + 1)
    for _ in range(100000):
        m = np.minimum(lab[e[:, 0]], lab[e[:, 1]])
        new = lab.copy()
        np.minimum.at(new, e[:, 0], m)
        np.minimum.at(new, e[:, 1], m)
        new = new[new]
        if np.array_equal(new, lab):
            break
        lab = new
    return lab[uid]


# ----------------------------------------------------------------------------------------------------------------------
# 2. landmarks
# ----------------------------------------------------------------------------------------------------------------------

def x_clusters(xs, gap=0.03, min_frac=0.02):
    """Sorted x values -> list of (lo, hi, count) runs separated by gaps, dropping tiny runs."""
    xs = np.sort(xs)
    if len(xs) == 0:
        return []
    br = np.where(np.diff(xs) > gap)[0]
    out, s = [], 0
    for b in list(br) + [len(xs) - 1]:
        if b - s + 1 >= max(3, min_frac * len(xs)):
            out.append((float(xs[s]), float(xs[b]), int(b - s + 1)))
        s = b + 1
    return out


def slab(v, z0, z1):
    return v[(v[:, 2] >= z0) & (v[:, 2] < z1)]


def arm_select(v, edge, sgn, arm_lo, arm_hi, neck_z, cy, margin=-0.06):
    """Vertices of one arm: outside the torso edge between the arm floor and the raised hand's top. Above the
    collar only the part clearly outside the head (far out and in front of the face) counts, so the head, its
    ear discs and the collar never enter the arm cloud while the raised hand does."""
    out = sgn * (v[:, 0] - edge)
    m = (out > margin) & (v[:, 2] > arm_lo) & (v[:, 2] < arm_hi)
    high = v[:, 2] > neck_z - 0.15
    m &= ~high | ((out > 0.12) & (v[:, 1] < cy - 0.35))
    return m


def find_landmarks(body, orb):
    """Joint positions (Blender Z-up world units) from the vertex clouds of the body and the orb."""
    H = float(body[:, 2].max())
    J = {}
    # torso centre from a slab around 30 % of the height (below the arms, above the sash tail)
    t = slab(body, 0.28 * H, 0.33 * H)
    cx = float((np.percentile(t[:, 0], 3) + np.percentile(t[:, 0], 97)) / 2)
    cy = float((np.percentile(t[:, 1], 3) + np.percentile(t[:, 1], 97)) / 2)
    # crotch: lowest slab where the two leg clusters (the ones straddling cx) have merged
    crotch = None
    legs_lo = slab(body, 0.0, 0.10 * H)
    leg_x0, leg_x1 = float(legs_lo[:, 0].min()) - 0.05, float(legs_lo[:, 0].max()) + 0.05
    prev_two = False
    z = 0.08 * H
    while z < 0.4 * H:
        s = slab(body, z, z + 0.02 * H)
        s = s[(s[:, 0] > leg_x0) & (s[:, 0] < leg_x1)]
        cl = [c for c in x_clusters(s[:, 0]) if c[0] < cx + 0.3 and c[1] > cx - 0.3]
        two = len(cl) >= 2
        if prev_two and not two:
            crotch = z
            break
        prev_two = two
        z += 0.02 * H
    if crotch is None:
        crotch = 0.18 * H
    # legs: side by x relative to the crotch centre, slices below the crotch
    legs = slab(body, 0.0, crotch)
    legs = legs[(legs[:, 0] > leg_x0) & (legs[:, 0] < leg_x1)]
    split_x = float(np.median(legs[:, 0]))
    # the median is inside one of the legs when the stance is asymmetric: use the largest gap instead
    xs = np.sort(legs[:, 0])
    gaps = np.diff(xs)
    gi = int(np.argmax(gaps))
    if gaps[gi] > 0.02:
        split_x = float((xs[gi] + xs[gi + 1]) / 2)
    for side, sel in (('L', legs[:, 0] > split_x), ('R', legs[:, 0] <= split_x)):
        leg = legs[sel]
        ankle_z = 0.075 * H
        knee_z = (crotch + ankle_z) / 2
        hip = slab(leg, crotch - 0.08 * H, crotch)[:, :2].mean(0)
        knee = slab(leg, knee_z - 0.03, knee_z + 0.03)[:, :2].mean(0)
        ankle = slab(leg, ankle_z - 0.03, ankle_z + 0.03)[:, :2].mean(0)
        foot = slab(leg, 0.0, 0.03 * H)
        toe = np.array([foot[:, 0].mean(), foot[:, 1].min() + 0.02, 0.015 * H])
        J['hip_' + side] = [float(hip[0]), float(hip[1]), crotch + 0.02 * H]
        J['knee_' + side] = [float(knee[0]), float(knee[1]), knee_z]
        J['ankle_' + side] = [float(ankle[0]), float(ankle[1]), ankle_z]
        J['toe_' + side] = [float(toe[0]), float(toe[1]), float(toe[2])]
    # neck: narrowest central cluster between 45 % and 75 % of the height
    # (width = 2nd..98th percentile of x inside a column window around the torso centre, so the raised hand and
    # the arms, which sit outside the window, do not count)
    best = None
    z = 0.45 * H
    while z < 0.75 * H:
        s = slab(body, z, z + 0.015 * H)
        s = s[np.abs(s[:, 0] - cx) < 0.15 * H]
        if len(s) > 20:
            lo, hi = np.percentile(s[:, 0], 2), np.percentile(s[:, 0], 98)
            if best is None or hi - lo < best[0]:
                best = (hi - lo, z + 0.0075 * H, (lo + hi) / 2)
        z += 0.015 * H
    neck_z, neck_x = best[1], best[2]
    ns = slab(body, neck_z - 0.03, neck_z + 0.03)
    ns = ns[np.abs(ns[:, 0] - neck_x) < 0.3]
    neck_y = float((np.percentile(ns[:, 1], 3) + np.percentile(ns[:, 1], 97)) / 2)
    # head: the dome centre from the slab above the neck (hand excluded by y: it is in front of the body)
    head = body[(body[:, 2] > neck_z + 0.2 * (H - neck_z)) & (body[:, 2] < neck_z + 0.7 * (H - neck_z))]
    head = head[head[:, 1] > cy - 0.55 * (H - neck_z)]
    hx, hy = float(head[:, 0].mean()), float(head[:, 1].mean())
    head_top_z = neck_z + 0.75 * (H - neck_z)
    # torso chain
    torso_len = neck_z - crotch
    J['root'] = [0.0, 0.0, 0.0]
    J['root_tip'] = [0.0, 0.0, 0.08 * H]
    J['pelvis'] = [cx, cy, crotch]
    J['spine'] = [cx, cy, crotch + 0.22 * torso_len]
    J['chest'] = [cx, cy, crotch + 0.50 * torso_len]
    J['neck_base'] = [cx * 0.5 + neck_x * 0.5, cy * 0.5 + neck_y * 0.5, crotch + 0.86 * torso_len]
    J['neck'] = [neck_x, neck_y, neck_z]
    J['head_top'] = [hx, hy, head_top_z]
    # shoulders at the torso edge, at 75 % of the torso
    sh_z = crotch + 0.75 * torso_len
    half = 0.32 * torso_len / 1.09 * 1.0 if torso_len > 0 else 0.3
    half = max(0.2, min(0.45, half))
    arm_lo, arm_hi = crotch + 0.45 * torso_len, neck_z + 0.2
    for side, sgn in (('L', 1.0), ('R', -1.0)):
        edge = cx + sgn * half
        arm = body[arm_select(body, edge, sgn, arm_lo, arm_hi, neck_z, cy, margin=0.0)]
        # the shoulder joint sits at the torso edge, at the torso's front/back centre of that height
        band = slab(body, sh_z - 0.05, sh_z + 0.05)
        band = band[np.abs(band[:, 0] - cx) < 0.25]
        sh_y = float((np.percentile(band[:, 1], 3) + np.percentile(band[:, 1], 97)) / 2) if len(band) > 20 else cy
        J['clavicle_' + side] = [cx + sgn * 0.12, sh_y, sh_z]
        J['shoulder_' + side] = [edge, sh_y, sh_z]
        # elbow: the lowest part of the arm cloud (both arms bend with the elbow down)
        outer = arm[sgn * (arm[:, 0] - edge) > 0.05]
        low = outer[outer[:, 2] < outer[:, 2].min() + 0.1]
        elbow = np.array([low[:, 0].mean(), low[:, 1].mean(), low[:, 2].mean() + 0.04])
        # hand: the part of the arm farthest from the elbow; the fingertip is its far end (the pointing finger on
        # the right, the open palm's fingers on the left), the wrist sits 0.14 before the hand's centre
        dist = np.linalg.norm(arm - elbow, axis=1)
        fingertip = arm[dist > dist.max() - 0.05].mean(0)
        hand_c = arm[dist > dist.max() - 0.25].mean(0)
        v = hand_c - elbow
        wrist = hand_c - 0.14 * v / (np.linalg.norm(v) + 1e-9)
        J['elbow_' + side] = [float(x) for x in elbow]
        J['wrist_' + side] = [float(x) for x in wrist]
        J['fingertip_' + side] = [float(x) for x in fingertip]
    oc = orb.mean(0) if len(orb) else np.array([cx + 0.7, cy, neck_z])
    J['orb'] = [float(oc[0]), float(oc[1]), float(oc[2])]
    J['orb_top'] = [float(oc[0]), float(oc[1]), float(oc[2]) + 0.15]
    meta = {'height': H, 'torso_centre': [cx, cy], 'crotch_z': crotch, 'neck_z': neck_z, 'shoulder_z': sh_z,
            'leg_x_range': [leg_x0, leg_x1], 'leg_split_x': split_x, 'arm_edge_half_width': half,
            'arm_z_range': [arm_lo, arm_hi]}
    return J, meta


def apply_overrides(J, path):
    with open(path) as f:
        data = json.load(f)
    src = data.get('joints', data)
    n = 0
    for k, v in src.items():
        if k in J and isinstance(v, (list, tuple)) and len(v) == 3:
            J[k] = [float(x) for x in v]
            n += 1
    log(f'landmarks overridden from {path}: {n} joints')


# ----------------------------------------------------------------------------------------------------------------------
# 3. armature + 4. weights
# ----------------------------------------------------------------------------------------------------------------------

def build_armature(J):
    arm = bpy.data.armatures.new('UbiRig')
    ao = bpy.data.objects.new('UbiRig', arm)
    bpy.context.scene.collection.objects.link(ao)
    bpy.context.view_layer.objects.active = ao
    ao.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT')
    eb = {}
    for b in BONES:
        e = arm.edit_bones.new(b)
        h, t = BONE_JOINTS[b]
        e.head = Vector(J[h])
        e.tail = Vector(J[t])
        if (e.tail - e.head).length < 0.02:
            e.tail = e.head + Vector((0, 0, 0.05))
        e.use_connect = False
        e.use_deform = b != 'Root'
        eb[b] = e
    for b in BONES:
        if PARENT[b]:
            eb[b].parent = eb[PARENT[b]]
    bpy.ops.object.mode_set(mode='OBJECT')
    return ao


def seg_dist(P, A, B):
    """Distance from points P (n,3) to segment AB."""
    AB = B - A
    L2 = float(AB @ AB)
    t = np.clip(((P - A) @ AB) / (L2 if L2 > 1e-12 else 1.0), 0.0, 1.0)
    Q = A + t[:, None] * AB
    return np.linalg.norm(P - Q, axis=1)


def smoothstep(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3 - 2 * x)


def compute_weights(co, orb_mask, J, meta):
    """
    Rigid-with-blend skinning.
    - the orb island is 100 % Orb;
    - every other vertex is put in a body region from the same cuts that produced the landmarks (left/right arm,
      left/right leg, core), and gets the nearest bone SEGMENT among that region's bones (regions keep the raised
      hand away from the head and the sash away from the arms);
    - near the joint between the chosen bone and its parent (or one of its children) the weight is shared with a
      smooth falloff inside BLEND_RADIUS.
    Returns a dict bone -> weight array (n,), rows sum to 1.
    """
    n = len(co)
    seg = {b: (np.array(J[BONE_JOINTS[b][0]]), np.array(J[BONE_JOINTS[b][1]])) for b in BONES}
    cx, cy = meta['torso_centre']
    H, crotch, neck_z, sh_z = meta['height'], meta['crotch_z'], meta['neck_z'], meta['shoulder_z']
    half = meta['arm_edge_half_width']
    x, y, z = co[:, 0], co[:, 1], co[:, 2]
    leg_x0, leg_x1 = meta['leg_x_range']
    split_x = meta['leg_split_x']

    region = np.full(n, 0, dtype=np.int8)  # 0 core, 1 arm L, 2 arm R, 3 leg L, 4 leg R, 5 orb
    arm_lo, arm_hi = meta['arm_z_range']
    for side, sgn, code in (('L', 1.0, 1), ('R', -1.0, 2)):
        edge = cx + sgn * half
        region[arm_select(co, edge, sgn, arm_lo, arm_hi, neck_z, cy)] = code
    legm = (z < crotch + 0.06) & (x > leg_x0) & (x < leg_x1)
    region[legm & (x > split_x)] = 3
    region[legm & (x <= split_x)] = 4
    region[orb_mask] = 5

    cand = {
        0: ['Hips', 'Spine', 'Chest', 'Neck', 'Head', 'Shoulder.L', 'Shoulder.R'],
        1: ['Chest', 'Shoulder.L', 'UpperArm.L', 'LowerArm.L', 'Hand.L'],
        2: ['Chest', 'Shoulder.R', 'UpperArm.R', 'LowerArm.R', 'Hand.R'],
        3: ['Hips', 'UpperLeg.L', 'LowerLeg.L', 'Foot.L'],
        4: ['Hips', 'UpperLeg.R', 'LowerLeg.R', 'Foot.R'],
        5: ['Orb'],
    }
    primary = np.empty(n, dtype=object)
    for code, names in cand.items():
        idx = np.where(region == code)[0]
        if len(idx) == 0:
            continue
        P = co[idx]
        D = np.stack([seg_dist(P, *seg[b]) for b in names], axis=1)
        best = np.argmin(D, axis=1)
        primary[idx] = np.array(names, dtype=object)[best]

    W = {b: np.zeros(n) for b in BONES}
    children = {b: [c for c in BONES if PARENT[c] == b] for b in BONES}
    for b in BONES:
        idx = np.where(primary == b)[0]
        if len(idx) == 0:
            continue
        P = co[idx]
        w_other = np.zeros(len(idx))
        other = np.empty(len(idx), dtype=object)
        # joint with the parent: this bone's head
        pairs = []
        if PARENT[b] and PARENT[b] != 'Root' and BLEND_RADIUS.get(b, 0) > 0:
            pairs.append((PARENT[b], np.array(J[BONE_JOINTS[b][0]]), BLEND_RADIUS[b]))
        for c in children[b]:
            if BLEND_RADIUS.get(c, 0) > 0:
                pairs.append((c, np.array(J[BONE_JOINTS[c][0]]), BLEND_RADIUS[c]))
        for ob, joint, R in pairs:
            d = np.linalg.norm(P - joint, axis=1)
            w = 0.5 * (1.0 - smoothstep(d / R))
            take = w > w_other
            w_other[take] = w[take]
            other[take] = ob
        W[b][idx] += 1.0 - w_other
        for ob in set(o for o in other if o is not None):
            sel = other == ob
            W[ob][idx[sel]] += w_other[sel]
    total = sum(W.values())
    assert np.all(total > 0.999), 'unweighted vertices'
    for b in BONES:
        W[b] /= total
    counts = {b: int((primary == b).sum()) for b in BONES}
    return W, counts


def apply_weights(mesh, ao, W):
    me = mesh.data
    for b in BONES:
        w = W[b]
        idx = np.where(w > 1e-4)[0]
        if len(idx) == 0:
            continue
        vg = mesh.vertex_groups.new(name=b)
        q = np.round(w[idx] * 255) / 255
        for val in np.unique(q):
            sel = idx[q == val].tolist()
            vg.add(sel, float(val), 'REPLACE')
    mesh.parent = ao
    mesh.matrix_parent_inverse = Matrix.Identity(4)
    mod = mesh.modifiers.new('Armature', 'ARMATURE')
    mod.object = ao
    mod.use_vertex_groups = True


# ----------------------------------------------------------------------------------------------------------------------
# 5. actions
# ----------------------------------------------------------------------------------------------------------------------

class Poser:
    """Keyframes pose bones from world-space (armature-space) rotations and translations."""

    def __init__(self, ao):
        self.ao = ao
        self.rest = {b: ao.data.bones[b].matrix_local.to_3x3() for b in BONES}
        # last quaternion keyed per bone in the current action: consecutive keys are kept sign-continuous (q and -q
        # are the same rotation, but the component-wise F-curves would interpolate through zero and the exported
        # samples would flip sign mid-way)
        self.prev = {}
        for pb in ao.pose.bones:
            pb.rotation_mode = 'QUATERNION'

    def reset(self):
        """Forget the previous keys (call when a new action starts)."""
        self.prev = {}

    def local_quat(self, bone, q_world):
        R = self.rest[bone]
        return (R.inverted() @ q_world.to_matrix() @ R).to_quaternion()

    def local_vec(self, bone, v_world):
        return self.rest[bone].inverted() @ Vector(v_world)

    def key(self, bone, frame, rot=None, loc=None, scale=None):
        pb = self.ao.pose.bones[bone]
        if rot is not None:
            q = self.local_quat(bone, rot)
            prev = self.prev.get(bone)
            if prev is not None and q.dot(prev) < 0:
                q.negate()
            self.prev[bone] = q.copy()
            pb.rotation_quaternion = q
            pb.keyframe_insert('rotation_quaternion', frame=frame)
        if loc is not None:
            pb.location = self.local_vec(bone, loc)
            pb.keyframe_insert('location', frame=frame)
        if scale is not None:
            pb.scale = Vector(scale)
            pb.keyframe_insert('scale', frame=frame)


# world axes (Blender): X = character's left, Y = back (front is -Y), Z = up
def rot(pitch=0.0, yaw=0.0, roll=0.0):
    """pitch > 0 nods forward/down, yaw > 0 turns to the character's left, roll > 0 tilts the top to the left."""
    q = Quaternion((0, 0, 1), math.radians(yaw))
    q = q @ Quaternion((1, 0, 0), math.radians(-pitch))
    q = q @ Quaternion((0, 1, 0), math.radians(roll))
    return q


def axis_rot(axis, deg):
    return Quaternion(Vector(axis).normalized(), math.radians(deg))


def wave_axis(J, side):
    """Axis that swings the forearm sideways (perpendicular to the forearm and to the left/right axis)."""
    f = Vector(J['wrist_' + side]) - Vector(J['elbow_' + side])
    a = f.cross(Vector((1, 0, 0)))
    if a.length < 1e-6:
        a = Vector((0, 1, 0))
    return a.normalized()


def ease_curves(action, cyclic):
    for slot in action.slots:
        cb = action.layers[0].strips[0].channelbag(slot)
        if cb is None:
            continue
        for fc in cb.fcurves:
            for k in fc.keyframe_points:
                k.interpolation = 'BEZIER'
                k.handle_left_type = 'AUTO_CLAMPED'
                k.handle_right_type = 'AUTO_CLAMPED'
            if cyclic:
                m = fc.modifiers.new('CYCLES')
                m.mode_before = 'REPEAT'
                m.mode_after = 'REPEAT'
            fc.update()


def author_actions(ao, J):
    P = Poser(ao)
    ao.animation_data_create()
    S = lambda t: math.sin(2 * math.pi * t)  # noqa: E731
    C = lambda t: math.cos(2 * math.pi * t)  # noqa: E731
    wax_R = wave_axis(J, 'R')
    orb0 = Vector((0, 0, 0))

    def begin(name, frames):
        act = bpy.data.actions.new(name)
        ao.animation_data.action = act
        P.reset()
        for pb in ao.pose.bones:
            pb.rotation_quaternion = (1, 0, 0, 0)
            pb.location = (0, 0, 0)
            pb.scale = (1, 1, 1)
        return act

    def finish(act, frames, cyclic):
        act.use_frame_range = True
        act.frame_start = 0
        act.frame_end = frames
        ease_curves(act, cyclic)
        ao.animation_data.action = None
        tr = ao.animation_data.nla_tracks.new()
        tr.name = act.name
        st = tr.strips.new(act.name, 0, act)
        st.action_slot = act.slots[0]
        tr.mute = True

    def loop(name, seconds, step, fn):
        frames = int(round(seconds * FPS))
        act = begin(name, frames)
        for f in range(0, frames + 1, step):
            t = (f % frames) / frames  # last key == first key
            fn(f, t)
        finish(act, frames, True)

    def oneshot(name, seconds, keys):
        """keys: list of (time_fraction, {bone: dict(rot=, loc=, scale=)}). Rest is keyed at both ends."""
        frames = int(round(seconds * FPS))
        act = begin(name, frames)
        touched = set(b for _, pose in keys for b in pose)
        for b in touched:
            P.key(b, 0, rot=Quaternion(), loc=(0, 0, 0), scale=(1, 1, 1))
        for tf, pose in keys:
            f = int(round(tf * frames))
            for b, kw in pose.items():
                P.key(b, f, **kw)
        for b in touched:
            P.key(b, frames, rot=Quaternion(), loc=(0, 0, 0), scale=(1, 1, 1))
        finish(act, frames, False)

    # --- Idle: breathing, tiny weight shift, slow head micro-sway, orb bobbing/orbiting -------------------------------
    def idle(f, t):
        P.key('Chest', f, rot=rot(pitch=2.0 * S(t)))
        P.key('Spine', f, rot=rot(pitch=1.0 * S(t)))
        P.key('Hips', f, rot=rot(roll=1.2 * S(t)), loc=(0.008 * S(t), 0, 0.004 * S(2 * t)))
        P.key('Neck', f, rot=rot(yaw=1.0 * S(t + 0.15)))
        P.key('Head', f, rot=rot(yaw=3.0 * S(t + 0.15), pitch=1.2 * S(2 * t + 0.3), roll=1.0 * S(t + 0.6)))
        P.key('UpperArm.L', f, rot=rot(pitch=1.0 * S(t + 0.2)))
        P.key('UpperArm.R', f, rot=rot(pitch=-1.0 * S(t + 0.1)))
        P.key('Orb', f, loc=orb0 + Vector((0.03 * C(t), 0.03 * S(t), 0.05 * S(2 * t))))
    loop('Idle', 4.0, 2, idle)

    # --- Yes: nod ---------------------------------------------------------------------------------------------------
    oneshot('Yes', 1.2, [
        (0.25, {'Head': dict(rot=rot(pitch=12)), 'Neck': dict(rot=rot(pitch=5))}),
        (0.50, {'Head': dict(rot=rot(pitch=-3)), 'Neck': dict(rot=rot(pitch=-1))}),
        (0.75, {'Head': dict(rot=rot(pitch=9)), 'Neck': dict(rot=rot(pitch=4))}),
    ])

    # --- No: head shake ---------------------------------------------------------------------------------------------
    oneshot('No', 1.2, [
        (0.17, {'Head': dict(rot=rot(yaw=12)), 'Neck': dict(rot=rot(yaw=4))}),
        (0.42, {'Head': dict(rot=rot(yaw=-12)), 'Neck': dict(rot=rot(yaw=-4))}),
        (0.67, {'Head': dict(rot=rot(yaw=12)), 'Neck': dict(rot=rot(yaw=4))}),
        (0.87, {'Head': dict(rot=rot(yaw=-9)), 'Neck': dict(rot=rot(yaw=-3))}),
    ])

    # --- Wave: the raised right forearm/hand swings sideways, small head tilt ------------------------------------------
    # The hand rests next to the face, so the wave goes outward (away from the head) and back, never across it.
    def wv(a):
        return axis_rot(wax_R, -a)  # a > 0 swings the forearm outward (towards -X)
    oneshot('Wave', 2.0, [
        (0.12, {'LowerArm.R': dict(rot=wv(20)), 'Hand.R': dict(rot=wv(-4)), 'UpperArm.R': dict(rot=wv(4)),
                'Head': dict(rot=rot(roll=-5)), 'Neck': dict(rot=rot(roll=-2))}),
        (0.30, {'LowerArm.R': dict(rot=wv(2)), 'Hand.R': dict(rot=wv(4))}),
        (0.47, {'LowerArm.R': dict(rot=wv(20)), 'Hand.R': dict(rot=wv(-4)), 'Head': dict(rot=rot(roll=-6, yaw=3))}),
        (0.64, {'LowerArm.R': dict(rot=wv(2)), 'Hand.R': dict(rot=wv(4))}),
        (0.80, {'LowerArm.R': dict(rot=wv(15)), 'Hand.R': dict(rot=wv(-3)), 'UpperArm.R': dict(rot=wv(3)),
                'Head': dict(rot=rot(roll=-3)), 'Neck': dict(rot=rot(roll=-1))}),
    ])

    # --- Jump: anticipation squat, hop up 0.25, squash on landing; arms pump ------------------------------------------
    def sq(y):
        s = 1 / math.sqrt(y)
        return (s, y, s)
    oneshot('Jump', 1.0, [
        (0.20, {'Hips': dict(loc=(0, 0, -0.07)), 'Spine': dict(rot=rot(pitch=8), scale=sq(0.93)),
                'Chest': dict(rot=rot(pitch=4)), 'Head': dict(rot=rot(pitch=4)),
                'UpperLeg.L': dict(rot=rot(pitch=15)), 'LowerLeg.L': dict(rot=rot(pitch=-28)), 'Foot.L': dict(rot=rot(pitch=13)),
                'UpperLeg.R': dict(rot=rot(pitch=15)), 'LowerLeg.R': dict(rot=rot(pitch=-28)), 'Foot.R': dict(rot=rot(pitch=13)),
                'UpperArm.L': dict(rot=rot(pitch=-14)), 'UpperArm.R': dict(rot=rot(pitch=-14))}),
        (0.50, {'Hips': dict(loc=(0, 0, 0.25)), 'Spine': dict(rot=rot(pitch=-5), scale=sq(1.06)),
                'Chest': dict(rot=rot(pitch=-3)), 'Head': dict(rot=rot(pitch=-6)),
                'UpperLeg.L': dict(rot=rot(pitch=10)), 'LowerLeg.L': dict(rot=rot(pitch=-22)), 'Foot.L': dict(rot=rot(pitch=-12)),
                'UpperLeg.R': dict(rot=rot(pitch=10)), 'LowerLeg.R': dict(rot=rot(pitch=-22)), 'Foot.R': dict(rot=rot(pitch=-12)),
                'UpperArm.L': dict(rot=rot(pitch=18)), 'UpperArm.R': dict(rot=rot(pitch=18)),
                'Orb': dict(loc=(0, 0, 0.12))}),
        (0.80, {'Hips': dict(loc=(0, 0, -0.05)), 'Spine': dict(rot=rot(pitch=6), scale=sq(0.95)),
                'Chest': dict(rot=rot(pitch=3)), 'Head': dict(rot=rot(pitch=3)),
                'UpperLeg.L': dict(rot=rot(pitch=10)), 'LowerLeg.L': dict(rot=rot(pitch=-20)), 'Foot.L': dict(rot=rot(pitch=10)),
                'UpperLeg.R': dict(rot=rot(pitch=10)), 'LowerLeg.R': dict(rot=rot(pitch=-20)), 'Foot.R': dict(rot=rot(pitch=10)),
                'UpperArm.L': dict(rot=rot(pitch=-6)), 'UpperArm.R': dict(rot=rot(pitch=-6)),
                'Orb': dict(loc=(0, 0, -0.04))}),
    ])

    # --- Excited: energetic bounce, arms pumping, orb orbiting fast ----------------------------------------------------
    def excited(f, t):
        b = abs(S(t))
        P.key('Hips', f, loc=(0, 0, 0.06 * b), rot=rot(roll=1.5 * S(t)))
        P.key('Spine', f, rot=rot(pitch=3.0 * S(2 * t)))
        P.key('Chest', f, rot=rot(pitch=2.0 * S(2 * t + 0.15)))
        P.key('Neck', f, rot=rot(yaw=1.5 * S(t)))
        P.key('Head', f, rot=rot(yaw=4.0 * S(t), pitch=-2.0 + 3.0 * S(2 * t + 0.25)))
        P.key('UpperArm.L', f, rot=rot(pitch=6.0 * S(2 * t)))
        P.key('UpperArm.R', f, rot=rot(pitch=6.0 * S(2 * t + 0.5)))
        P.key('LowerArm.R', f, rot=axis_rot(wax_R, 8.0 * S(2 * t + 0.2)))
        P.key('Hand.L', f, rot=rot(pitch=-6.0 * S(2 * t + 0.1)))
        P.key('Orb', f, loc=orb0 + Vector((0.06 * C(t), 0.06 * S(t), 0.05 * S(2 * t) + 0.03)))
    loop('Excited', 1.2, 1, excited)

    # --- Worried: shoulders up, fast small shiver on head/chest, orb pulled close --------------------------------------
    def worried(f, t):
        sh = 6 * t  # 4 Hz shiver over the 1.5 s loop
        P.key('Shoulder.L', f, rot=rot(roll=-12))
        P.key('Shoulder.R', f, rot=rot(roll=12))
        P.key('Chest', f, rot=rot(pitch=4 + 0.8 * S(sh), yaw=0.5 * S(sh + 0.25)))
        P.key('Neck', f, rot=rot(pitch=2))
        P.key('Head', f, rot=rot(pitch=6 + 0.8 * S(sh + 0.1), yaw=1.5 * S(sh), roll=1.0 * S(sh + 0.4)))
        P.key('UpperArm.L', f, rot=rot(pitch=4 + 0.5 * S(sh)))
        P.key('UpperArm.R', f, rot=rot(pitch=4 + 0.5 * S(sh + 0.5)))
        P.key('Orb', f, loc=orb0 + Vector((-0.11 + 0.01 * S(sh), 0.0, -0.14 + 0.012 * S(sh + 0.3))))
    loop('Worried', 1.5, 1, worried)

    # --- Sleep: slumped chest, head down/side, very slow breathing, orb drifting low ----------------------------------
    def sleep(f, t):
        P.key('Spine', f, rot=rot(pitch=4 + 0.5 * S(t)))
        P.key('Chest', f, rot=rot(pitch=12 + 1.5 * S(t)))
        P.key('Neck', f, rot=rot(pitch=3, roll=2))
        P.key('Head', f, rot=rot(pitch=12 + 1.0 * S(t), roll=8, yaw=-4))
        P.key('Shoulder.L', f, rot=rot(roll=4))
        P.key('Shoulder.R', f, rot=rot(roll=-4))
        P.key('UpperArm.L', f, rot=rot(pitch=3 + 0.4 * S(t)))
        P.key('UpperArm.R', f, rot=rot(pitch=3 + 0.4 * S(t)))
        P.key('Orb', f, loc=orb0 + Vector((-0.05, 0.02, -0.25 + 0.03 * S(t))))
    loop('Sleep', 4.0, 2, sleep)

    names = [a.name for a in bpy.data.actions]
    assert sorted(names) == sorted(CLIPS), names


# ----------------------------------------------------------------------------------------------------------------------
# 6. export, post-process, optimise, verify
# ----------------------------------------------------------------------------------------------------------------------

def export_glb(path):
    bpy.ops.export_scene.gltf(
        filepath=path, export_format='GLB', export_apply=True, export_skins=True, export_animations=True,
        export_animation_mode='ACTIONS', export_yup=True, export_texcoords=True, export_normals=True,
        export_materials='EXPORT', export_image_format='AUTO', export_force_sampling=True,
        export_reset_pose_bones=True, export_rest_position_armature=True, export_def_bones=False,
        export_optimize_animation_size=True, export_anim_single_armature=True, export_frame_range=False,
        export_influence_nb=4, export_all_influences=False, export_extras=False, export_leaf_bone=False,
    )


def read_glb(path):
    d = open(path, 'rb').read()
    assert d[:4] == b'glTF'
    n = struct.unpack('<I', d[12:16])[0]
    j = json.loads(d[20:20 + n])
    off = 20 + n
    bn = struct.unpack('<I', d[off:off + 4])[0]
    assert d[off + 4:off + 8] == b'BIN\x00'
    return j, d[off + 8:off + 8 + bn]


def write_glb(path, j, binb):
    js = json.dumps(j, separators=(',', ':')).encode()
    js += b' ' * ((4 - len(js) % 4) % 4)
    binb = bytes(binb) + b'\x00' * ((4 - len(binb) % 4) % 4)
    total = 12 + 8 + len(js) + 8 + len(binb)
    with open(path, 'wb') as f:
        f.write(b'glTF' + struct.pack('<II', 2, total))
        f.write(struct.pack('<I', len(js)) + b'JSON' + js)
        f.write(struct.pack('<I', len(binb)) + b'BIN\x00' + binb)


def accessor_array(j, binb, idx):
    acc = j['accessors'][idx]
    bv = j['bufferViews'][acc['bufferView']]
    ncomp = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[acc['type']]
    dt = {5126: np.float32, 5123: np.uint16, 5121: np.uint8, 5125: np.uint32}[acc['componentType']]
    start = bv.get('byteOffset', 0) + acc.get('byteOffset', 0)
    a = np.frombuffer(binb, dtype=dt, count=acc['count'] * ncomp, offset=start)
    return a.reshape(acc['count'], ncomp)


def make_rotation_keys_continuous(j, binb, anim, ch):
    """
    Negates the rotation keys of `ch` whose dot with the previous key is negative (q and -q are the same rotation).
    The exporter writes rest · pose per sample with w >= 0, so a bone whose rest rotation sits near w = 0 (the legs:
    their rest is a ~180 degree turn from the parent) flips sign whenever the pose crosses that point. three's slerp
    takes the short arc either way, but additive blending or another engine would show a spin. The first key keeps
    the sign of the node's rest rotation so a one-shot still starts (and ends) at the rest value verbatim.
    """
    s = anim['samplers'][ch['sampler']]
    acc = j['accessors'][s['output']]
    if acc['componentType'] != 5126 or acc['type'] != 'VEC4':
        return 0
    q = accessor_array(j, binb, s['output']).astype(np.float64).copy()
    rest = np.array(j['nodes'][ch['target']['node']].get('rotation', [0, 0, 0, 1]))
    flipped = 0
    prev = rest
    for i in range(len(q)):
        if np.dot(q[i], prev) < 0:
            q[i] = -q[i]
            flipped += 1
        prev = q[i]
    if flipped:
        bv = j['bufferViews'][acc['bufferView']]
        start = bv.get('byteOffset', 0) + acc.get('byteOffset', 0)
        raw = q.astype(np.float32).tobytes()
        binb[start:start + len(raw)] = raw
        acc.pop('min', None)
        acc.pop('max', None)
    return flipped


def strip_rest_channels(path):
    """
    Post-processes the animations in place: removes the channels that only repeat the node's rest transform (the
    exporter keys every bone) and makes the rotation keys sign-continuous.
    """
    j, binb = read_glb(path)
    binb = bytearray(binb)
    removed = 0
    flipped = 0
    for anim in j.get('animations', []):
        keep_ch, keep_s, remap = [], [], {}
        for ch in anim['channels']:
            node = j['nodes'][ch['target']['node']]
            s = anim['samplers'][ch['sampler']]
            out = accessor_array(j, binb, s['output']).astype(np.float64)
            p = ch['target']['path']
            rest = {'translation': node.get('translation', [0, 0, 0]), 'rotation': node.get('rotation', [0, 0, 0, 1]),
                    'scale': node.get('scale', [1, 1, 1])}[p]
            same = np.all(np.abs(out - np.array(rest)) < 1e-5)
            if p == 'rotation':
                same = same or np.all(np.abs(out + np.array(rest)) < 1e-5)
            if same:
                removed += 1
                continue
            if ch['sampler'] not in remap:
                remap[ch['sampler']] = len(keep_s)
                keep_s.append(s)
            if p == 'rotation':
                flipped += make_rotation_keys_continuous(j, binb, anim, ch)
            ch['sampler'] = remap[ch['sampler']]
            keep_ch.append(ch)
        anim['channels'], anim['samplers'] = keep_ch, keep_s
    write_glb(path, j, binb)
    log(f'stripped {removed} rest-only animation channels, negated {flipped} rotation keys for sign continuity')


def summarize(path):
    j, binb = read_glb(path)
    names = {i: nd.get('name') for i, nd in enumerate(j['nodes'])}
    skins = [[names[i] for i in s['joints']] for s in j.get('skins', [])]
    anims = {}
    for a in j.get('animations', []):
        tmax = max(float(j['accessors'][s['input']]['max'][0]) for s in a['samplers']) if a['samplers'] else 0
        bones = sorted(set(names[c['target']['node']] for c in a['channels']))
        paths = sorted(set(names[c['target']['node']] + ':' + c['target']['path'] for c in a['channels']))
        # consecutive rotation keys with a negative dot product (q → -q): the same rotation, but a consumer that
        # blends component-wise (additive layers, some engines) would show a spin
        flips = 0
        for c in a['channels']:
            if c['target']['path'] != 'rotation':
                continue
            q = accessor_array(j, binb, a['samplers'][c['sampler']]['output']).astype(np.float64)
            flips += int(np.sum(np.sum(q[1:] * q[:-1], axis=1) < 0))
        anims[a['name']] = {'seconds': round(tmax, 3), 'bones': bones, 'channels': len(a['channels']),
                            'root_translation': 'Root:translation' in paths, 'rotation_flips': flips}
    prims = [p for m in j.get('meshes', []) for p in m['primitives']]
    attrs = sorted(set(k for p in prims for k in p['attributes']))
    imgs = []
    for im in j.get('images', []):
        bv = j['bufferViews'][im['bufferView']]
        imgs.append({'mime': im.get('mimeType'), 'bytes': bv['byteLength']})
    ext = j.get('extensionsRequired', [])
    return {'file': path, 'bytes': os.path.getsize(path), 'skins': skins, 'animations': anims, 'attributes': attrs,
            'primitives': len(prims), 'images': imgs, 'extensionsRequired': ext,
            'materials': [m.get('name') for m in j.get('materials', [])]}


def gt(args):
    env = dict(os.environ, NODE_USE_ENV_PROXY='1')
    cmd = ['pnpm', 'dlx', '@gltf-transform/cli@4'] + args
    log('$', ' '.join(cmd))
    r = subprocess.run(cmd, env=env, capture_output=True, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stdout + r.stderr)
        raise SystemExit('gltf-transform failed: ' + ' '.join(args))
    return r.stdout


def optimise(src, out, texture_size, tmpdir):
    a = os.path.join(tmpdir, 'o1.glb')
    b = os.path.join(tmpdir, 'o2.glb')
    c = os.path.join(tmpdir, 'o3.glb')
    d = os.path.join(tmpdir, 'o4.glb')
    gt(['prune', src, a])
    gt(['dedup', a, b])
    gt(['weld', b, c])  # merges vertices with identical attributes (JOINTS/WEIGHTS included), no visual change
    gt(['resize', '--width', str(texture_size), '--height', str(texture_size), c, d])
    nodraco = out[:-4] + '-nodraco.glb' if out.endswith('.glb') else out + '-nodraco.glb'
    shutil.copyfile(d, nodraco)
    gt(['draco', '--method', 'edgebreaker', d, out])
    return nodraco


def verify(path, max_bytes):
    s = summarize(path)
    problems = []
    if len(s['skins']) != 1:
        problems.append(f"expected 1 skin, got {len(s['skins'])}")
    elif sorted(s['skins'][0]) != sorted(BONES):
        problems.append(f"joints differ from the contract: {s['skins'][0]}")
    if sorted(s['animations']) != sorted(CLIPS):
        problems.append(f"animations differ from the contract: {sorted(s['animations'])}")
    for k in ('JOINTS_0', 'WEIGHTS_0', 'POSITION', 'NORMAL', 'TEXCOORD_0'):
        if k not in s['attributes']:
            problems.append(f'missing attribute {k}')
    for n, a in s['animations'].items():
        if a['root_translation'] and n not in ('Jump', 'Excited'):
            problems.append(f'{n} translates Root')
        if a['rotation_flips']:
            problems.append(f"{n} has {a['rotation_flips']} quaternion sign flips between consecutive keys")
    if s['bytes'] > max_bytes:
        problems.append(f"{s['bytes']} bytes > {max_bytes}")
    return s, problems


# ----------------------------------------------------------------------------------------------------------------------
# 7. renders
# ----------------------------------------------------------------------------------------------------------------------

def setup_render(J, meta, res=512, samples=24):
    sc = bpy.context.scene
    sc.render.engine = 'CYCLES'
    sc.cycles.device = 'CPU'
    sc.cycles.samples = samples
    sc.cycles.use_denoising = False
    sc.render.resolution_x = sc.render.resolution_y = res
    sc.render.resolution_percentage = 100
    sc.render.image_settings.file_format = 'PNG'
    sc.view_settings.view_transform = 'Standard'
    world = bpy.data.worlds.new('World')
    sc.world = world
    world.use_nodes = True
    bg = world.node_tree.nodes['Background']
    bg.inputs[0].default_value = (0.55, 0.6, 0.7, 1)
    bg.inputs[1].default_value = 0.7
    sun = bpy.data.lights.new('Sun', 'SUN')
    sun.energy = 3.0
    sun.angle = math.radians(8)
    so = bpy.data.objects.new('Sun', sun)
    sc.collection.objects.link(so)
    so.rotation_euler = (math.radians(50), math.radians(-15), math.radians(-35))
    cam = bpy.data.cameras.new('Cam')
    cam.lens = 50
    co = bpy.data.objects.new('Cam', cam)
    sc.collection.objects.link(co)
    sc.camera = co
    frame_camera(meta)


def frame_camera(meta, closeup=False):
    """Camera in front, slightly above; `closeup` frames the head, shoulders and hands."""
    co = bpy.context.scene.camera
    H = meta['height']
    cx, cy = meta['torso_centre']
    if closeup:
        target = Vector((cx, cy, 0.58 * H))
        co.location = Vector((cx + 0.15, cy - 0.85 * H, 0.66 * H))
    else:
        target = Vector((cx, cy, 0.52 * H))
        co.location = Vector((cx + 0.3, cy - 1.7 * H, 0.66 * H))
    d = target - co.location
    co.rotation_euler = d.to_track_quat('-Z', 'Y').to_euler()


def render(path, meta=None, closeup=False):
    sc = bpy.context.scene
    if meta is not None:
        frame_camera(meta, closeup)
    sc.render.filepath = path
    t = time.time()
    bpy.ops.render.render(write_still=True)
    log(f'rendered {path} in {time.time() - t:.1f}s')


def render_set(ao, J, meta, out_dir):
    os.makedirs(out_dir, exist_ok=True)
    setup_render(J, meta)
    sc = bpy.context.scene
    P = Poser(ao)
    ad = ao.animation_data

    def rest():
        ad.action = None
        for pb in ao.pose.bones:
            pb.rotation_quaternion = (1, 0, 0, 0)
            pb.location = (0, 0, 0)
            pb.scale = (1, 1, 1)
        sc.frame_set(0)

    rest()
    render(os.path.join(out_dir, '01-rest.png'), meta)
    rest()
    # the app splits the look-at 70 % head / 30 % neck
    ao.pose.bones['Head'].rotation_quaternion = P.local_quat('Head', rot(yaw=35 * 0.7, pitch=-20 * 0.7))
    ao.pose.bones['Neck'].rotation_quaternion = P.local_quat('Neck', rot(yaw=35 * 0.3, pitch=-20 * 0.3))
    render(os.path.join(out_dir, '02-head-yaw35-pitch-20.png'), meta)
    render(os.path.join(out_dir, '02b-head-yaw35-pitch-20-closeup.png'), meta, closeup=True)
    for name, frame, fname, close in (('Wave', 6, '03-wave-peak.png', False), ('Wave', 6, '03b-wave-peak-closeup.png', True),
                                      ('Sleep', 48, '04-sleep-48.png', False),
                                      ('Excited', 7, '05-excited-mid.png', False), ('Jump', 12, '06-jump-apex.png', False),
                                      ('Worried', 10, '07-worried.png', True)):
        rest()
        act = bpy.data.actions[name]
        ad.action = act
        ad.action_slot = act.slots[0]
        sc.frame_set(frame)
        render(os.path.join(out_dir, fname), meta, closeup=close)
    rest()


# ----------------------------------------------------------------------------------------------------------------------
# main
# ----------------------------------------------------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('raw')
    ap.add_argument('out')
    ap.add_argument('--renders', help='directory for Cycles validation renders')
    ap.add_argument('--landmarks', help='JSON with joint overrides (same shape as <out>.landmarks.json)')
    ap.add_argument('--no-optimize', action='store_true', help='skip gltf-transform (raw Blender export only)')
    ap.add_argument('--texture-size', type=int, default=1024)
    ap.add_argument('--max-bytes', type=int, default=8 * 1024 * 1024)
    ap.add_argument('--blend', help='also save the Blender scene here (.blend)')
    ap.add_argument('--keep-temp', action='store_true')
    args = ap.parse_args(sys.argv[1:] if not sys.argv[0].endswith('blender') else sys.argv[sys.argv.index('--') + 1:])

    t0 = time.time()
    out = os.path.abspath(args.out)
    os.makedirs(os.path.dirname(out), exist_ok=True)
    tmpdir = out + '.tmp'
    os.makedirs(tmpdir, exist_ok=True)

    mesh = import_raw(os.path.abspath(args.raw))
    co, ev = mesh_arrays(mesh)
    lab = islands(co, ev)
    ids, cnt = np.unique(lab, return_counts=True)
    body_id = ids[np.argmax(cnt)]
    orb_mask = lab != body_id
    body, orb = co[~orb_mask], co[orb_mask]
    log(f'mesh: {len(co)} vertices, {len(ids)} islands, body {len(body)}, orb {len(orb)}, height {body[:, 2].max():.3f}')

    J, meta = find_landmarks(body, orb)
    if args.landmarks:
        apply_overrides(J, args.landmarks)
    lm_path = out[:-4] + '.landmarks.json' if out.endswith('.glb') else out + '.landmarks.json'
    with open(lm_path, 'w') as f:
        json.dump({'joints': J, 'meta': meta, 'bones': {b: list(BONE_JOINTS[b]) for b in BONES}}, f, indent=1)
    log('landmarks (Blender Z-up world units, x = character left, -y = front):')
    for k, v in J.items():
        log(f'  {k:14s} {v[0]:+.3f} {v[1]:+.3f} {v[2]:+.3f}')
    log('meta', json.dumps(meta))

    ao = build_armature(J)
    W, counts = compute_weights(co, orb_mask, J, meta)
    log('primary vertex counts per bone: ' + ', '.join(f'{b}={counts[b]}' for b in BONES))
    apply_weights(mesh, ao, W)
    author_actions(ao, J)
    log(f'rig + clips ready in {time.time() - t0:.1f}s')

    if args.blend:
        bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(args.blend))

    raw_export = os.path.join(tmpdir, 'export.glb')
    t = time.time()
    export_glb(raw_export)
    log(f'exported {raw_export} ({os.path.getsize(raw_export) / 1e6:.1f} MB) in {time.time() - t:.1f}s')
    strip_rest_channels(raw_export)

    if args.no_optimize:
        shutil.copyfile(raw_export, out)
        nodraco = None
    else:
        t = time.time()
        nodraco = optimise(raw_export, out, args.texture_size, tmpdir)
        log(f'optimised in {time.time() - t:.1f}s')

    summary, problems = verify(out, args.max_bytes)
    with open(out[:-4] + '.summary.json' if out.endswith('.glb') else out + '.summary.json', 'w') as f:
        json.dump(summary, f, indent=1)
    log('summary: ' + json.dumps({k: summary[k] for k in ('bytes', 'skins', 'attributes', 'images', 'extensionsRequired')}))
    for n, a in summary['animations'].items():
        log(f"  clip {n:8s} {a['seconds']:.3f}s {a['channels']:3d} channels, bones: {', '.join(a['bones'])}")
    if nodraco:
        log(f'uncompressed copy: {nodraco} ({os.path.getsize(nodraco) / 1e6:.1f} MB)')

    if args.renders:
        render_set(ao, J, meta, os.path.abspath(args.renders))

    if not args.keep_temp:
        shutil.rmtree(tmpdir, ignore_errors=True)
    if problems:
        for p in problems:
            log('PROBLEM:', p)
        raise SystemExit(1)
    log(f'done: {out} ({os.path.getsize(out) / 1e6:.2f} MB) in {time.time() - t0:.1f}s')


if __name__ == '__main__':
    main()
