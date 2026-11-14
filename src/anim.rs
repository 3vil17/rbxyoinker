// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use rbx_dom_weak::WeakDom;
use rbx_types::{Ref, Variant};
use serde_json::{json, Value};
use std::collections::HashMap;

const FPS: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf {
    r: [[f32; 3]; 3],
    p: [f32; 3],
}

impl Xf {
    const ID: Xf = Xf { r: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], p: [0.0; 3] };

    fn mul(&self, o: &Xf) -> Xf {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.r[i][k] * o.r[k][j]).sum();
            }
        }
        Xf { r, p: add(self.apply_dir(o.p), self.p) }
    }

    fn inv(&self) -> Xf {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = self.r[j][i];
            }
        }
        let t = Xf { r, p: [0.0; 3] };
        let p = t.apply_dir(self.p);
        Xf { r, p: [-p[0], -p[1], -p[2]] }
    }

    fn apply_dir(&self, v: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|i| self.r[i][0] * v[0] + self.r[i][1] * v[1] + self.r[i][2] * v[2])
    }

    fn apply(&self, v: [f32; 3]) -> [f32; 3] {
        add(self.apply_dir(v), self.p)
    }

    fn quat(&self) -> [f32; 4] {
        let m = &self.r;
        let tr = m[0][0] + m[1][1] + m[2][2];
        let q = if tr > 0.0 {
            let s = (tr + 1.0).sqrt() * 2.0;
            [(m[2][1] - m[1][2]) / s, (m[0][2] - m[2][0]) / s, (m[1][0] - m[0][1]) / s, 0.25 * s]
        } else if m[0][0] > m[1][1] && m[0][0] > m[2][2] {
            let s = (1.0 + m[0][0] - m[1][1] - m[2][2]).sqrt() * 2.0;
            [0.25 * s, (m[0][1] + m[1][0]) / s, (m[0][2] + m[2][0]) / s, (m[2][1] - m[1][2]) / s]
        } else if m[1][1] > m[2][2] {
            let s = (1.0 + m[1][1] - m[0][0] - m[2][2]).sqrt() * 2.0;
            [(m[0][1] + m[1][0]) / s, 0.25 * s, (m[1][2] + m[2][1]) / s, (m[0][2] - m[2][0]) / s]
        } else {
            let s = (1.0 + m[2][2] - m[0][0] - m[1][1]).sqrt() * 2.0;
            [(m[0][2] + m[2][0]) / s, (m[1][2] + m[2][1]) / s, 0.25 * s, (m[1][0] - m[0][1]) / s]
        };
        normalize4(q)
    }

    fn from_quat(q: [f32; 4], p: [f32; 3]) -> Xf {
        let [x, y, z, w] = q;
        Xf {
            r: [
                [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - z * w), 2.0 * (x * z + y * w)],
                [2.0 * (x * y + z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - x * w)],
                [2.0 * (x * z - y * w), 2.0 * (y * z + x * w), 1.0 - 2.0 * (x * x + y * y)],
            ],
            p,
        }
    }

    fn from_cframe(cf: &rbx_types::CFrame) -> Xf {
        let o = &cf.orientation;
        Xf {
            r: [[o.x.x, o.x.y, o.x.z], [o.y.x, o.y.y, o.y.z], [o.z.x, o.z.y, o.z.z]],
            p: [cf.position.x, cf.position.y, cf.position.z],
        }
    }

    fn matrix(&self) -> [f32; 16] {
        let r = &self.r;
        [
            r[0][0], r[1][0], r[2][0], 0.0, r[0][1], r[1][1], r[2][1], 0.0, r[0][2], r[1][2], r[2][2], 0.0, self.p[0], self.p[1],
            self.p[2], 1.0,
        ]
    }
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn normalize4(q: [f32; 4]) -> [f32; 4] {
    let l = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    if l < 1e-8 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    q.map(|v| v / l)
}

fn slerp(a: [f32; 4], mut b: [f32; 4], t: f32) -> [f32; 4] {
    let mut d: f32 = (0..4).map(|i| a[i] * b[i]).sum();
    if d < 0.0 {
        b = b.map(|v| -v);
        d = -d;
    }
    if d > 0.9995 {
        return normalize4([0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t));
    }
    let th = d.acos();
    let s = th.sin();
    let (wa, wb) = (((1.0 - t) * th).sin() / s, (t * th).sin() / s);
    normalize4([0, 1, 2, 3].map(|i| a[i] * wa + b[i] * wb))
}

fn lerp_xf(a: &Xf, b: &Xf, t: f32) -> Xf {
    let p = [0, 1, 2].map(|i| a.p[i] + (b.p[i] - a.p[i]) * t);
    Xf::from_quat(slerp(a.quat(), b.quat(), t), p)
}

fn bounce_out(t: f32) -> f32 {
    let (n, d) = (7.5625, 2.75);
    if t < 1.0 / d {
        n * t * t
    } else if t < 2.0 / d {
        let t = t - 1.5 / d;
        n * t * t + 0.75
    } else if t < 2.5 / d {
        let t = t - 2.25 / d;
        n * t * t + 0.9375
    } else {
        let t = t - 2.625 / d;
        n * t * t + 0.984375
    }
}

fn elastic_out(t: f32) -> f32 {
    if t <= 0.0 || t >= 1.0 {
        return t.clamp(0.0, 1.0);
    }
    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * (2.0 * std::f32::consts::PI / 3.0)).sin() + 1.0
}

fn cubic_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease(style: u32, direction: u32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let out: fn(f32) -> f32 = match style {
        0 => return t,
        1 => return 0.0,
        2 => elastic_out,
        4 => bounce_out,
        _ => cubic_out,
    };
    let legacy = matches!(style, 2..=4);
    let direction = match (legacy, direction) {
        (true, 0) => 1,
        (true, 1) => 0,
        (_, d) => d,
    };
    match direction {
        0 => 1.0 - out(1.0 - t),
        1 => out(t),
        _ => {
            if t < 0.5 {
                (1.0 - out(1.0 - 2.0 * t)) / 2.0
            } else {
                (1.0 + out(2.0 * t - 1.0)) / 2.0
            }
        }
    }
}

fn prop<'a>(dom: &'a WeakDom, r: Ref, names: &[&str]) -> Option<&'a Variant> {
    let inst = dom.get_by_ref(r)?;
    names.iter().find_map(|n| inst.properties.get(&rbx_dom_weak::ustr(n)))
}

fn cframe(dom: &WeakDom, r: Ref, names: &[&str]) -> Option<Xf> {
    match prop(dom, r, names)? {
        Variant::CFrame(cf) => Some(Xf::from_cframe(cf)),
        Variant::OptionalCFrame(Some(cf)) => Some(Xf::from_cframe(cf)),
        _ => None,
    }
}

fn vec3(dom: &WeakDom, r: Ref, names: &[&str]) -> Option<[f32; 3]> {
    match prop(dom, r, names)? {
        Variant::Vector3(v) => Some([v.x, v.y, v.z]),
        _ => None,
    }
}

fn reference(dom: &WeakDom, r: Ref, name: &str) -> Option<Ref> {
    match prop(dom, r, &[name])? {
        Variant::Ref(x) if !x.is_none() => Some(*x),
        _ => None,
    }
}

fn number(dom: &WeakDom, r: Ref, name: &str) -> Option<f32> {
    match prop(dom, r, &[name])? {
        Variant::Float32(v) => Some(*v),
        Variant::Float64(v) => Some(*v as f32),
        Variant::Int32(v) => Some(*v as f32),
        _ => None,
    }
}

fn enum_value(dom: &WeakDom, r: Ref, name: &str) -> u32 {
    match prop(dom, r, &[name]) {
        Some(Variant::Enum(e)) => e.to_u32(),
        _ => 0,
    }
}

fn asset_id(v: &Variant) -> Option<i64> {
    let s = match v {
        Variant::ContentId(c) => c.as_str().to_string(),
        Variant::Content(c) => c.as_uri()?.to_string(),
        Variant::String(s) => s.clone(),
        _ => return None,
    };
    let digits: String = s.chars().rev().take_while(|c| c.is_ascii_digit()).collect::<Vec<_>>().into_iter().rev().collect();
    digits.parse().ok().filter(|id| *id > 0)
}

fn color(dom: &WeakDom, r: Ref) -> [f32; 4] {
    match prop(dom, r, &["Color", "Color3uint8"]) {
        Some(Variant::Color3uint8(c)) => [c.r as f32 / 255.0, c.g as f32 / 255.0, c.b as f32 / 255.0, 1.0],
        Some(Variant::Color3(c)) => [c.r, c.g, c.b, 1.0],
        _ => [0.64, 0.64, 0.64, 1.0],
    }
}

fn is_part(class: &str) -> bool {
    matches!(class, "Part" | "MeshPart" | "UnionOperation" | "WedgePart" | "CornerWedgePart" | "TrussPart")
}

enum Shape {
    Box,
    Mesh { id: i64, scale: Option<[f32; 3]> },
}

struct PartInfo {
    name: String,
    cf: Xf,
    size: [f32; 3],
    shape: Shape,
    color: [f32; 4],
    visible: bool,
    texture: Option<i64>,
}

fn part_info(dom: &WeakDom, r: Ref) -> Option<PartInfo> {
    let inst = dom.get_by_ref(r)?;
    if !is_part(inst.class.as_str()) {
        return None;
    }
    let size = vec3(dom, r, &["Size", "size"]).unwrap_or([1.0; 3]);
    let mut shape = Shape::Box;
    let mut texture = None;
    if inst.class.as_str() == "MeshPart" {
        if let Some(id) = prop(dom, r, &["MeshContent", "MeshId"]).and_then(asset_id) {
            shape = Shape::Mesh { id, scale: None };
        }
        texture = prop(dom, r, &["TextureContent", "TextureID"]).and_then(asset_id);
    }
    for c in inst.children() {
        let Some(child) = dom.get_by_ref(*c) else { continue };
        match child.class.as_str() {
            "SpecialMesh" => {
                if let Some(id) = prop(dom, *c, &["MeshId"]).and_then(asset_id) {
                    shape = Shape::Mesh { id, scale: Some(vec3(dom, *c, &["Scale"]).unwrap_or([1.0; 3])) };
                }
                if let Some(id) = prop(dom, *c, &["TextureId"]).and_then(asset_id) {
                    texture = Some(id);
                }
            }
            "SurfaceAppearance" => {
                if let Some(id) = prop(dom, *c, &["ColorMapContent", "ColorMap"]).and_then(asset_id) {
                    texture = Some(id);
                }
            }
            _ => {}
        }
    }
    let visible = number(dom, r, "Transparency").unwrap_or(0.0) < 0.99;
    Some(PartInfo { name: inst.name.clone(), cf: cframe(dom, r, &["CFrame"])?, size, shape, color: color(dom, r), visible, texture })
}

pub fn asset_ids(bytes: &[u8]) -> (Vec<i64>, Vec<i64>) {
    let Ok(dom) = rbx_binary::from_reader(bytes) else { return (Vec::new(), Vec::new()) };
    let mut meshes = Vec::new();
    let mut textures = Vec::new();
    for inst in dom.descendants() {
        if let Some(p) = part_info(&dom, inst.referent()).filter(|p| p.visible) {
            if let Shape::Mesh { id, .. } = p.shape {
                if !meshes.contains(&id) {
                    meshes.push(id);
                }
            }
            if let Some(id) = p.texture.filter(|id| !textures.contains(id)) {
                textures.push(id);
            }
        }
    }
    (meshes, textures)
}

struct Bone {
    name: String,
    parent: Option<usize>,
    world: Xf,
    motor: Option<(Xf, Xf)>,
}

struct Prim {
    bone: usize,
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    idx: Vec<u32>,
    uv: Vec<[f32; 2]>,
    skin: Option<Vec<([u16; 4], [f32; 4])>>,
    color: [f32; 4],
    texture: Option<i64>,
}

struct Geo {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    idx: Vec<u32>,
    uv: Vec<[f32; 2]>,
    skin: Option<Vec<([u16; 4], [u8; 4])>>,
    names: Vec<String>,
}

struct Track {
    keys: Vec<(f32, Xf, u32, u32)>,
}

impl Track {
    fn sample(&self, t: f32) -> Xf {
        let k = &self.keys;
        if t <= k[0].0 {
            return k[0].1;
        }
        let last = k.len() - 1;
        if t >= k[last].0 {
            return k[last].1;
        }
        let i = k.iter().rposition(|key| key.0 <= t).unwrap_or(0);
        let (a, b) = (&k[i], &k[i + 1]);
        let span = (b.0 - a.0).max(1e-6);
        lerp_xf(&a.1, &b.1, ease(a.2, a.3, (t - a.0) / span))
    }
}

struct Clip {
    name: String,
    length: f32,
    tracks: HashMap<String, Track>,
}

fn collect_poses(dom: &WeakDom, r: Ref, time: f32, tracks: &mut HashMap<String, Track>) {
    let Some(inst) = dom.get_by_ref(r) else { return };
    for c in inst.children() {
        let Some(child) = dom.get_by_ref(*c) else { continue };
        if child.class.as_str() != "Pose" {
            continue;
        }
        let weight = number(dom, *c, "Weight").unwrap_or(1.0);
        if weight > 0.0 {
            if let Some(cf) = cframe(dom, *c, &["CFrame"]) {
                let key = (time, cf, enum_value(dom, *c, "EasingStyle"), enum_value(dom, *c, "EasingDirection"));
                tracks.entry(child.name.clone()).or_insert(Track { keys: Vec::new() }).keys.push(key);
            }
        }
        collect_poses(dom, *c, time, tracks);
    }
}

fn clips(dom: &WeakDom) -> Vec<Clip> {
    let mut out: Vec<Clip> = Vec::new();
    for seq in dom.descendants().filter(|i| i.class.as_str() == "KeyframeSequence") {
        let mut tracks = HashMap::new();
        let mut length: f32 = 0.0;
        for k in seq.children() {
            let Some(kf) = dom.get_by_ref(*k) else { continue };
            if kf.class.as_str() != "Keyframe" {
                continue;
            }
            let time = number(dom, *k, "Time").unwrap_or(0.0);
            length = length.max(time);
            collect_poses(dom, *k, time, &mut tracks);
        }
        for t in tracks.values_mut() {
            t.keys.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
        if tracks.is_empty() {
            continue;
        }
        let mut name = seq.name.clone();
        let mut n = 2;
        while out.iter().any(|c| c.name == name) {
            name = format!("{} {n}", seq.name);
            n += 1;
        }
        out.push(Clip { name, length, tracks });
    }
    out
}

fn box_prim(size: [f32; 3]) -> Geo {
    let h = size.map(|s| s / 2.0);
    let mut pos = Vec::new();
    let mut nrm = Vec::new();
    let mut idx = Vec::new();
    let mut uv = Vec::new();
    for axis in 0..3 {
        for sign in [1.0f32, -1.0] {
            let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
            let base = pos.len() as u32;
            for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let mut p = [0.0; 3];
                p[axis] = sign * h[axis];
                p[u] = a * h[u];
                p[v] = b * h[v];
                pos.push(p);
                let mut n = [0.0; 3];
                n[axis] = sign;
                nrm.push(n);
                uv.push([(a + 1.0) / 2.0, (1.0 - b) / 2.0]);
            }
            if sign > 0.0 {
                idx.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
            } else {
                idx.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
            }
        }
    }
    Geo { pos, nrm, idx, uv, skin: None, names: Vec::new() }
}

fn mesh_prim(mesh: &rbxmesh::Mesh, size: [f32; 3], scale: Option<[f32; 3]>) -> Option<Geo> {
    let faces = mesh.lod_faces(0)?;
    if mesh.vertices.is_empty() || faces.is_empty() {
        return None;
    }
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for v in &mesh.vertices {
        for i in 0..3 {
            lo[i] = lo[i].min(v.position[i]);
            hi[i] = hi[i].max(v.position[i]);
        }
    }
    let center = [0, 1, 2].map(|i| (lo[i] + hi[i]) / 2.0);
    let k = match scale {
        Some(s) => s,
        None => [0, 1, 2].map(|i| if hi[i] - lo[i] > 1e-6 { size[i] / (hi[i] - lo[i]) } else { 1.0 }),
    };
    let off = if scale.is_some() { [0.0; 3] } else { center };
    let pos = mesh.vertices.iter().map(|v| [0, 1, 2].map(|i| (v.position[i] - off[i]) * k[i])).collect();
    let nrm = mesh
        .vertices
        .iter()
        .map(|v| {
            let n = [0, 1, 2].map(|i| v.normal[i] / k[i].abs().max(1e-6));
            let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
            n.map(|x| x / l)
        })
        .collect();
    let idx = faces.iter().flat_map(|f| f.iter().copied()).filter(|i| (*i as usize) < mesh.vertices.len()).collect::<Vec<_>>();
    let names = mesh.bones.iter().map(|b| b.name.clone()).collect();
    let uv = mesh.vertices.iter().map(|v| v.uv).collect();
    (idx.len() % 3 == 0).then_some(Geo { pos, nrm, idx, uv, skin: mesh.joints(), names })
}

pub struct Export {
    pub glb: Vec<u8>,
    pub bones: usize,
    pub clips: Vec<String>,
}

pub fn build(bytes: &[u8], meshes: &HashMap<i64, Vec<u8>>, textures: &HashMap<i64, Vec<u8>>) -> Result<Export, String> {
    let dom = rbx_binary::from_reader(bytes).map_err(|e| format!("couldn't read the rig: {e}"))?;
    let mut parts: HashMap<Ref, PartInfo> = HashMap::new();
    let mut motors: Vec<(Ref, Ref, Xf, Xf)> = Vec::new();
    let mut welds: Vec<(Ref, Ref)> = Vec::new();
    let mut root: Option<Ref> = None;
    for inst in dom.descendants() {
        let r = inst.referent();
        let class = inst.class.as_str();
        if let Some(p) = part_info(&dom, r) {
            if p.name == "HumanoidRootPart" && root.is_none() {
                root = Some(r);
            }
            parts.insert(r, p);
        } else if is_part(class) && inst.name == "HumanoidRootPart" && root.is_none() {
            root = Some(r);
        }
        match class {
            "Motor6D" | "Motor" => {
                if let (Some(a), Some(b)) = (reference(&dom, r, "Part0"), reference(&dom, r, "Part1")) {
                    let c0 = cframe(&dom, r, &["C0"]).unwrap_or(Xf::ID);
                    let c1 = cframe(&dom, r, &["C1"]).unwrap_or(Xf::ID);
                    motors.push((a, b, c0, c1));
                }
            }
            "Weld" | "ManualWeld" | "WeldConstraint" | "RigidConstraint" => {
                if let (Some(a), Some(b)) = (reference(&dom, r, "Part0"), reference(&dom, r, "Part1")) {
                    welds.push((a, b));
                }
            }
            _ => {}
        }
    }
    let root = root
        .or_else(|| motors.iter().map(|m| m.0).find(|p0| !motors.iter().any(|m| m.1 == *p0)))
        .ok_or("this rig has no HumanoidRootPart or Motor6D joints")?;
    let root_cf = cframe(&dom, root, &["CFrame"]).unwrap_or(Xf::ID);
    let root_name = dom.get_by_ref(root).map_or("HumanoidRootPart".to_string(), |i| i.name.clone());
    let origin = root_cf.inv();

    let mut bones = vec![Bone { name: root_name, parent: None, world: origin.mul(&root_cf), motor: None }];
    let mut bone_of: HashMap<Ref, usize> = HashMap::from([(root, 0)]);
    let mut frontier = vec![root];
    while let Some(p0) = frontier.pop() {
        for (a, b, c0, c1) in &motors {
            if *a != p0 || bone_of.contains_key(b) {
                continue;
            }
            let parent = bone_of[a];
            let world = bones[parent].world.mul(c0).mul(&c1.inv());
            let name = dom.get_by_ref(*b).map_or("Part".to_string(), |i| i.name.clone());
            bones.push(Bone { name, parent: Some(parent), world, motor: Some((*c0, c1.inv())) });
            bone_of.insert(*b, bones.len() - 1);
            frontier.push(*b);
        }
    }

    let mut attach: HashMap<Ref, (usize, Xf)> = HashMap::new();
    let mut bone_names: HashMap<String, usize> = HashMap::new();
    for (&r, &b) in &bone_of {
        if let Some(p) = parts.get(&r) {
            attach.insert(r, (b, bones[b].world.mul(&p.cf.inv())));
        }
    }
    let mut grew = true;
    while grew {
        grew = false;
        for (a, b) in &welds {
            for (known, other) in [(a, b), (b, a)] {
                if attach.contains_key(other) || !parts.contains_key(other) {
                    continue;
                }
                let Some(&(bone, fix)) = attach.get(known) else { continue };
                attach.insert(*other, (bone, fix));
                grew = true;
            }
        }
    }

    let mut pending: Vec<Ref> = dom
        .descendants()
        .filter(|i| i.class.as_str() == "Bone")
        .map(|i| i.referent())
        .collect();
    let mut placed: HashMap<Ref, usize> = HashMap::new();
    let mut progress = true;
    while progress && !pending.is_empty() {
        progress = false;
        pending.retain(|r| {
            let Some(inst) = dom.get_by_ref(*r) else { return false };
            let parent = inst.parent();
            let local = cframe(&dom, *r, &["CFrame"]).unwrap_or(Xf::ID);
            let base = if let Some(&b) = placed.get(&parent) {
                Some((b, bones[b].world))
            } else if let (Some(&(b, fix)), Some(p)) = (attach.get(&parent), parts.get(&parent)) {
                Some((b, fix.mul(&p.cf)))
            } else {
                None
            };
            let Some((joint, parent_world)) = base else { return true };
            let world = parent_world.mul(&local);
            let offset = bones[joint].world.inv().mul(&world);
            bones.push(Bone { name: inst.name.clone(), parent: Some(joint), world, motor: Some((offset, Xf::ID)) });
            placed.insert(*r, bones.len() - 1);
            progress = true;
            false
        });
    }
    for (i, b) in bones.iter().enumerate() {
        bone_names.entry(b.name.clone()).or_insert(i);
    }

    let mut prims = Vec::new();
    for (r, p) in &parts {
        let Some(&(bone, fix)) = attach.get(r) else { continue };
        if !p.visible {
            continue;
        }
        let geo = match &p.shape {
            Shape::Mesh { id, scale } => meshes
                .get(id)
                .and_then(|b| rbxmesh::read(b).ok())
                .and_then(|m| mesh_prim(&m, p.size, *scale))
                .unwrap_or_else(|| box_prim(p.size)),
            Shape::Box => box_prim(p.size),
        };
        let world = fix.mul(&p.cf);
        let pos = geo.pos.iter().map(|v| world.apply(*v)).collect();
        let nrm = geo.nrm.iter().map(|n| world.apply_dir(*n)).collect();
        let map: Vec<Option<usize>> = geo.names.iter().map(|n| bone_names.get(n).copied()).collect();
        let skin = geo.skin.filter(|_| map.iter().any(Option::is_some)).map(|joints| {
            joints
                .iter()
                .map(|(j, w)| {
                    let mut ids = [bone as u16; 4];
                    let mut ws = [0.0f32; 4];
                    for k in 0..4 {
                        if w[k] == 0 {
                            continue;
                        }
                        if let Some(Some(b)) = map.get(j[k] as usize) {
                            ids[k] = *b as u16;
                            ws[k] = w[k] as f32;
                        }
                    }
                    let total: f32 = ws.iter().sum();
                    if total <= 0.0 {
                        return ([bone as u16, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]);
                    }
                    (ids, ws.map(|x| x / total))
                })
                .collect()
        });
        let texture = p.texture.filter(|id| textures.contains_key(id));
        prims.push(Prim { bone, pos, nrm, idx: geo.idx, uv: geo.uv, skin, color: p.color, texture });
    }
    if prims.is_empty() {
        return Err("no visible parts are attached to the rig's joints".into());
    }
    prims.sort_by_key(|p| p.bone);

    let clips = clips(&dom);
    let glb = write_glb(&bones, &prims, &clips, textures);
    Ok(Export { glb, bones: bones.len(), clips: clips.iter().map(|c| c.name.clone()).collect() })
}

pub(crate) struct Buf {
    pub(crate) bin: Vec<u8>,
    pub(crate) views: Vec<Value>,
    pub(crate) accessors: Vec<Value>,
}

impl Buf {
    pub(crate) fn push(&mut self, data: &[u8], target: Option<u32>) -> usize {
        while self.bin.len() % 4 != 0 {
            self.bin.push(0);
        }
        let mut view = json!({ "buffer": 0, "byteOffset": self.bin.len(), "byteLength": data.len() });
        if let Some(t) = target {
            view["target"] = json!(t);
        }
        self.bin.extend_from_slice(data);
        self.views.push(view);
        self.views.len() - 1
    }

    pub(crate) fn floats(&mut self, data: &[f32], width: usize, kind: &str, target: Option<u32>, bounds: bool) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.push(&bytes, target);
        let mut acc = json!({ "bufferView": view, "componentType": 5126, "count": data.len() / width, "type": kind });
        if bounds {
            let mut lo = vec![f32::MAX; width];
            let mut hi = vec![f32::MIN; width];
            for chunk in data.chunks(width) {
                for i in 0..width {
                    lo[i] = lo[i].min(chunk[i]);
                    hi[i] = hi[i].max(chunk[i]);
                }
            }
            acc["min"] = json!(lo);
            acc["max"] = json!(hi);
        }
        self.accessors.push(acc);
        self.accessors.len() - 1
    }

    pub(crate) fn ints(&mut self, data: &[u32], target: u32) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.push(&bytes, Some(target));
        self.accessors.push(json!({ "bufferView": view, "componentType": 5125, "count": data.len(), "type": "SCALAR" }));
        self.accessors.len() - 1
    }

    fn joints(&mut self, data: &[u16]) -> usize {
        let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = self.push(&bytes, Some(34962));
        self.accessors.push(json!({ "bufferView": view, "componentType": 5123, "count": data.len() / 4, "type": "VEC4" }));
        self.accessors.len() - 1
    }
}

fn local_of(bones: &[Bone], i: usize) -> Xf {
    match bones[i].parent {
        Some(p) => bones[p].world.inv().mul(&bones[i].world),
        None => bones[i].world,
    }
}

fn write_glb(bones: &[Bone], prims: &[Prim], clips: &[Clip], textures: &HashMap<i64, Vec<u8>>) -> Vec<u8> {
    let mut buf = Buf { bin: Vec::new(), views: Vec::new(), accessors: Vec::new() };
    let mut nodes = vec![json!({ "name": "Armature", "rotation": [0.0, 1.0, 0.0, 0.0], "children": [1] })];
    for (i, b) in bones.iter().enumerate() {
        let l = local_of(bones, i);
        let kids: Vec<usize> = bones.iter().enumerate().filter(|(_, c)| c.parent == Some(i)).map(|(j, _)| j + 1).collect();
        let mut node = json!({ "name": b.name, "translation": l.p, "rotation": l.quat() });
        if !kids.is_empty() {
            node["children"] = json!(kids);
        }
        nodes.push(node);
    }
    let ibm: Vec<f32> = bones.iter().flat_map(|b| b.world.inv().matrix()).collect();
    let ibm_acc = buf.floats(&ibm, 16, "MAT4", None, false);

    let mut images: Vec<i64> = Vec::new();
    let mut image_json = Vec::new();
    let mut materials: Vec<([f32; 4], Option<usize>)> = Vec::new();
    let mut primitives = Vec::new();
    for p in prims {
        let flat_pos: Vec<f32> = p.pos.iter().flatten().copied().collect();
        let flat_nrm: Vec<f32> = p.nrm.iter().flatten().copied().collect();
        let (joints, weights): (Vec<u16>, Vec<f32>) = match &p.skin {
            Some(skin) => (skin.iter().flat_map(|s| s.0).collect(), skin.iter().flat_map(|s| s.1).collect()),
            None => (p.pos.iter().flat_map(|_| [p.bone as u16, 0, 0, 0]).collect(), p.pos.iter().flat_map(|_| [1.0f32, 0.0, 0.0, 0.0]).collect()),
        };
        let tex = p.texture.and_then(|id| {
            if let Some(i) = images.iter().position(|x| *x == id) {
                return Some(i);
            }
            let data = textures.get(&id)?;
            let mime = if data.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
            let view = buf.push(data, None);
            image_json.push(json!({ "bufferView": view, "mimeType": mime, "name": id.to_string() }));
            images.push(id);
            Some(images.len() - 1)
        });
        let key = (if tex.is_some() { [1.0; 4] } else { p.color }, tex);
        let mat = materials.iter().position(|m| *m == key).unwrap_or_else(|| {
            materials.push(key);
            materials.len() - 1
        });
        let flat_uv: Vec<f32> = p.uv.iter().flatten().copied().collect();
        let pos = buf.floats(&flat_pos, 3, "VEC3", Some(34962), true);
        let nrm = buf.floats(&flat_nrm, 3, "VEC3", Some(34962), false);
        let jnt = buf.joints(&joints);
        let wgt = buf.floats(&weights, 4, "VEC4", Some(34962), false);
        let idx = buf.ints(&p.idx, 34963);
        let uvs = buf.floats(&flat_uv, 2, "VEC2", Some(34962), false);
        primitives.push(json!({
            "attributes": { "POSITION": pos, "NORMAL": nrm, "TEXCOORD_0": uvs, "JOINTS_0": jnt, "WEIGHTS_0": wgt },
            "indices": idx,
            "material": mat,
        }));
    }
    nodes.push(json!({ "name": "Body", "mesh": 0, "skin": 0 }));
    let body = nodes.len() - 1;

    let mut animations = Vec::new();
    for clip in clips {
        let frames = ((clip.length * FPS).ceil() as usize).max(1) + 1;
        let times: Vec<f32> = (0..frames).map(|f| (f as f32 / FPS).min(clip.length.max(0.0))).collect();
        let input = buf.floats(&times, 1, "SCALAR", None, true);
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        for (i, b) in bones.iter().enumerate() {
            let (Some((c0, c1)), Some(track)) = (b.motor, clip.tracks.get(&b.name)) else { continue };
            let mut tr = Vec::with_capacity(frames * 3);
            let mut rot = Vec::with_capacity(frames * 4);
            let mut prev: Option<[f32; 4]> = None;
            for t in &times {
                let l = c0.mul(&track.sample(*t)).mul(&c1);
                tr.extend(l.p);
                let mut q = l.quat();
                if let Some(pq) = prev {
                    if (0..4).map(|k| pq[k] * q[k]).sum::<f32>() < 0.0 {
                        q = q.map(|v| -v);
                    }
                }
                prev = Some(q);
                rot.extend(q);
            }
            let t_out = buf.floats(&tr, 3, "VEC3", None, false);
            let r_out = buf.floats(&rot, 4, "VEC4", None, false);
            samplers.push(json!({ "input": input, "output": t_out, "interpolation": "LINEAR" }));
            channels.push(json!({ "sampler": samplers.len() - 1, "target": { "node": i + 1, "path": "translation" } }));
            samplers.push(json!({ "input": input, "output": r_out, "interpolation": "LINEAR" }));
            channels.push(json!({ "sampler": samplers.len() - 1, "target": { "node": i + 1, "path": "rotation" } }));
        }
        if !channels.is_empty() {
            animations.push(json!({ "name": clip.name, "samplers": samplers, "channels": channels }));
        }
    }

    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "rbxyoinker" },
        "scene": 0,
        "scenes": [{ "nodes": [0, body] }],
        "nodes": nodes,
        "meshes": [{ "name": "Body", "primitives": primitives }],
        "skins": [{ "joints": (1..=bones.len()).collect::<Vec<_>>(), "inverseBindMatrices": ibm_acc, "skeleton": 1 }],
        "materials": materials
            .iter()
            .map(|(c, tex)| {
                let mut pbr = json!({ "baseColorFactor": c, "metallicFactor": 0.0, "roughnessFactor": 0.8 });
                if let Some(t) = tex {
                    pbr["baseColorTexture"] = json!({ "index": t });
                }
                json!({ "pbrMetallicRoughness": pbr })
            })
            .collect::<Vec<_>>(),
        "bufferViews": buf.views,
        "accessors": buf.accessors,
    });
    while buf.bin.len() % 4 != 0 {
        buf.bin.push(0);
    }
    doc["buffers"] = json!([{ "byteLength": buf.bin.len() }]);
    if !animations.is_empty() {
        doc["animations"] = json!(animations);
    }
    if !image_json.is_empty() {
        doc["images"] = json!(image_json);
        doc["samplers"] = json!([{ "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 }]);
        doc["textures"] = json!((0..image_json.len()).map(|i| json!({ "source": i, "sampler": 0 })).collect::<Vec<_>>());
    }
    let mut text = serde_json::to_vec(&doc).unwrap_or_default();
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + buf.bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(text);
    out.extend((buf.bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(buf.bin);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: &Xf, b: &Xf) -> bool {
        (0..3).all(|i| (a.p[i] - b.p[i]).abs() < 1e-4 && (0..3).all(|j| (a.r[i][j] - b.r[i][j]).abs() < 1e-4))
    }

    #[test]
    fn inverse_and_quat_round_trip() {
        let a = Xf::from_quat(normalize4([0.3, -0.2, 0.5, 0.8]), [1.0, 2.0, -3.0]);
        assert!(close(&a.mul(&a.inv()), &Xf::ID));
        assert!(close(&Xf::from_quat(a.quat(), a.p), &a));
    }

    #[test]
    fn easing_ends() {
        for style in 0..6 {
            for dir in 0..3 {
                if style != 1 {
                    assert!((ease(style, dir, 1.0) - 1.0).abs() < 1e-3, "{style} {dir}");
                }
                assert!(ease(style, dir, 0.0).abs() < 1e-3, "{style} {dir}");
            }
        }
    }
}
