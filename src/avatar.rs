// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::anim::Buf;
use crate::pool::Pool;
use crate::roblox::Roblox;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub enum Target {
    User(i64),
    Asset(i64),
}

pub struct Avatar {
    pub glb: Vec<u8>,
    pub parts: usize,
    pub textures: usize,
    pub texture_total: usize,
}

pub fn cdn(file: &str) -> String {
    if file.starts_with("http") {
        return file.to_string();
    }
    let mut i: u32 = 31;
    for b in file.bytes() {
        i ^= b as u32;
    }
    format!("https://t{}.rbxcdn.com/{}", i % 8, file)
}

fn number_after(text: &str, key: &str) -> Option<i64> {
    let at = text.find(key)? + key.len();
    let digits: String = text[at..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok().filter(|n| *n > 0)
}

pub async fn resolve(rb: &Roblox, input: &str) -> Result<(Target, String), String> {
    let text = input.trim();
    let lower = text.to_ascii_lowercase();
    if let Some(id) = number_after(&lower, "/users/") {
        let name = rb.user_name(id).await.ok_or("couldn't find that user")?;
        return Ok((Target::User(id), name));
    }
    if let Some(id) = number_after(&lower, "/catalog/").or_else(|| number_after(&lower, "/library/")) {
        let name = rb.asset_details(id).await.map(|d| d.name).unwrap_or_else(|_| format!("Item {id}"));
        return Ok((Target::Asset(id), name));
    }
    if let Ok(id) = text.parse::<i64>() {
        let name = rb.user_name(id).await.ok_or("no user has that id")?;
        return Ok((Target::User(id), name));
    }
    let name = text.trim_start_matches('@');
    if name.is_empty() || name.len() > 20 || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("paste a username, a user id, a profile link or a catalog link".into());
    }
    let (id, real) = rb.user_by_name(name).await.ok_or("no user has that name")?;
    Ok((Target::User(id), real))
}

async fn manifest(rb: &Roblox, pool: &Pool, target: &Target) -> Result<Value, String> {
    let url = match target {
        Target::User(id) => format!("https://thumbnails.roblox.com/v1/users/avatar-3d?userId={id}"),
        Target::Asset(id) => format!("https://thumbnails.roblox.com/v1/assets-thumbnail-3d?assetId={id}"),
    };
    let lease = pool.acquire(&[]).await.ok_or("log in or add a cookie first, Roblox only gives 3D models to logged in accounts")?;
    for _ in 0..15 {
        let Some(v) = rb.get_json(&url, Some(&lease.cookie)).await else {
            return Err("Roblox didn't give a 3D model for that".into());
        };
        match v["state"].as_str().unwrap_or("") {
            "Completed" => {
                let link = v["imageUrl"].as_str().ok_or("Roblox sent an empty 3D model")?;
                return rb.get_json(link, None).await.ok_or_else(|| "couldn't read the 3D model".into());
            }
            "Pending" => tokio::time::sleep(std::time::Duration::from_millis(1500)).await,
            "Blocked" => return Err("Roblox blocked the 3D model for that".into()),
            other => return Err(format!("Roblox said the 3D model is {}", if other.is_empty() { "missing" } else { other })),
        }
    }
    Err("Roblox is still rendering it, try again in a minute".into())
}

pub async fn fetch(rb: &Roblox, pool: &Pool, target: &Target) -> Result<Avatar, String> {
    let meta = manifest(rb, pool, target).await?;
    let obj_file = meta["obj"].as_str().ok_or("the 3D model has no mesh")?;
    let mtl_file = meta["mtl"].as_str().ok_or("the 3D model has no materials")?;
    let (obj_url, mtl_url) = (cdn(obj_file), cdn(mtl_file));
    let (obj, mtl) = tokio::join!(rb.get_bytes(&obj_url, None), rb.get_bytes(&mtl_url, None));
    let obj = String::from_utf8_lossy(&obj.ok_or("couldn't download the mesh")?).into_owned();
    let mtl = String::from_utf8_lossy(&mtl.ok_or("couldn't download the materials")?).into_owned();
    let materials = parse_mtl(&mtl);
    let mut wanted: Vec<String> = materials.values().filter_map(|m| m.texture.clone()).collect();
    wanted.sort();
    wanted.dedup();
    let mut set = tokio::task::JoinSet::new();
    for file in wanted.iter().cloned() {
        let rb = rb.clone();
        set.spawn(async move {
            let got = rb.get_bytes(&cdn(&file), None).await;
            (file, got)
        });
    }
    let mut images = HashMap::new();
    while let Some(Ok((file, got))) = set.join_next().await {
        if let Some(b) = got {
            images.insert(file, b);
        }
    }
    let texture_total = wanted.len();
    let textures = images.len();
    let (glb, parts) = tokio::task::spawn_blocking(move || build(&obj, &materials, &images))
        .await
        .map_err(|_| "building the model crashed".to_string())??;
    Ok(Avatar { glb, parts, textures, texture_total })
}

#[derive(Clone, Default)]
pub struct Material {
    color: [f32; 3],
    alpha: f32,
    texture: Option<String>,
}

pub fn parse_mtl(text: &str) -> HashMap<String, Material> {
    let mut out = HashMap::new();
    let mut name = String::new();
    let mut cur = Material { color: [1.0; 3], alpha: 1.0, texture: None };
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("newmtl") => {
                if !name.is_empty() {
                    out.insert(name.clone(), cur.clone());
                }
                name = it.collect::<Vec<_>>().join(" ");
                cur = Material { color: [1.0; 3], alpha: 1.0, texture: None };
            }
            Some("Kd") => {
                let v: Vec<f32> = it.filter_map(|x| x.parse().ok()).collect();
                if v.len() >= 3 {
                    cur.color = [v[0], v[1], v[2]];
                }
            }
            Some("d") => cur.alpha = it.next().and_then(|x| x.parse().ok()).unwrap_or(1.0),
            Some("map_Kd") => cur.texture = it.last().map(str::to_string),
            _ => {}
        }
    }
    if !name.is_empty() {
        out.insert(name, cur);
    }
    out
}

#[derive(Default)]
struct Prim {
    pos: Vec<f32>,
    nrm: Vec<f32>,
    uv: Vec<f32>,
    idx: Vec<u32>,
    seen: HashMap<(usize, usize, usize), u32>,
}

fn index(raw: &str, len: usize) -> Option<usize> {
    let n: i64 = raw.parse().ok()?;
    let i = if n < 0 { len as i64 + n } else { n - 1 };
    (i >= 0 && (i as usize) < len).then_some(i as usize)
}

fn has_alpha(img: &[u8]) -> bool {
    img.len() > 25 && img.starts_with(b"\x89PNG") && matches!(img[25], 4 | 6)
}

pub fn build(obj: &str, materials: &HashMap<String, Material>, images: &HashMap<String, Vec<u8>>) -> Result<(Vec<u8>, usize), String> {
    let mut v: Vec<[f32; 3]> = Vec::new();
    let mut vn: Vec<[f32; 3]> = Vec::new();
    let mut vt: Vec<[f32; 2]> = Vec::new();
    let mut groups: Vec<(String, Vec<(String, Prim)>)> = Vec::new();
    let mut group = String::from("Avatar");
    let mut mat = String::new();
    for line in obj.lines() {
        let mut it = line.split_whitespace();
        let Some(tag) = it.next() else { continue };
        let nums = |it: std::str::SplitWhitespace| it.filter_map(|x| x.parse::<f32>().ok()).collect::<Vec<_>>();
        match tag {
            "v" => {
                let p = nums(it);
                if p.len() >= 3 {
                    v.push([p[0], p[1], p[2]]);
                }
            }
            "vn" => {
                let p = nums(it);
                if p.len() >= 3 {
                    vn.push([p[0], p[1], p[2]]);
                }
            }
            "vt" => {
                let p = nums(it);
                if p.len() >= 2 {
                    vt.push([p[0], 1.0 - p[1]]);
                }
            }
            "g" | "o" => group = it.collect::<Vec<_>>().join(" "),
            "usemtl" => mat = it.collect::<Vec<_>>().join(" "),
            "f" => {
                let mut corners = Vec::new();
                for c in it {
                    let mut p = c.split('/');
                    let Some(pi) = p.next().and_then(|x| index(x, v.len())) else { continue };
                    let ti = p.next().and_then(|x| index(x, vt.len())).map_or(usize::MAX, |i| i);
                    let ni = p.next().and_then(|x| index(x, vn.len())).map_or(usize::MAX, |i| i);
                    corners.push((pi, ti, ni));
                }
                if corners.len() < 3 {
                    continue;
                }
                if groups.last().map_or(true, |g| g.0 != group) {
                    groups.push((group.clone(), Vec::new()));
                }
                let prims = &mut groups.last_mut().expect("group").1;
                if prims.last().map_or(true, |p| p.0 != mat) {
                    prims.push((mat.clone(), Prim::default()));
                }
                let prim = &mut prims.last_mut().expect("prim").1;
                let mut ids = Vec::with_capacity(corners.len());
                for key in corners {
                    let next = prim.pos.len() as u32 / 3;
                    let id = *prim.seen.entry(key).or_insert_with(|| {
                        prim.pos.extend_from_slice(&v[key.0]);
                        prim.nrm.extend_from_slice(vn.get(key.2).unwrap_or(&[0.0, 1.0, 0.0]));
                        prim.uv.extend_from_slice(vt.get(key.1).unwrap_or(&[0.0, 0.0]));
                        next
                    });
                    ids.push(id);
                }
                for k in 1..ids.len() - 1 {
                    prim.idx.extend_from_slice(&[ids[0], ids[k], ids[k + 1]]);
                }
            }
            _ => {}
        }
    }
    if v.is_empty() || groups.is_empty() {
        return Err("the 3D model was empty".into());
    }
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in &v {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let shift = [-(lo[0] + hi[0]) / 2.0, -lo[1], -(lo[2] + hi[2]) / 2.0];

    let mut buf = Buf { bin: Vec::new(), views: Vec::new(), accessors: Vec::new() };
    let mut gl_images = Vec::new();
    let mut gl_textures = Vec::new();
    let mut tex_of: HashMap<String, (usize, bool)> = HashMap::new();
    let mut gl_materials = Vec::new();
    let mut mat_of: HashMap<String, usize> = HashMap::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut used = HashMap::new();
    for (gname, prims) in &groups {
        let mut gl_prims = Vec::new();
        for (mname, prim) in prims {
            if prim.idx.is_empty() {
                continue;
            }
            let material = match mat_of.get(mname) {
                Some(m) => *m,
                None => {
                    let m = materials.get(mname).cloned().unwrap_or(Material { color: [1.0; 3], alpha: 1.0, texture: None });
                    let mut pbr = json!({ "baseColorFactor": [m.color[0], m.color[1], m.color[2], m.alpha], "metallicFactor": 0.0, "roughnessFactor": 1.0 });
                    let mut cutout = false;
                    if let Some(file) = m.texture.as_ref().filter(|f| images.contains_key(*f)) {
                        let (t, alpha) = *tex_of.entry(file.clone()).or_insert_with(|| {
                            let img = &images[file];
                            let view = buf.push(img, None);
                            let mime = if img.starts_with(&[0xFF, 0xD8]) { "image/jpeg" } else { "image/png" };
                            gl_images.push(json!({ "bufferView": view, "mimeType": mime, "name": file }));
                            gl_textures.push(json!({ "source": gl_images.len() - 1, "sampler": 0 }));
                            (gl_textures.len() - 1, has_alpha(img))
                        });
                        pbr["baseColorFactor"] = json!([1.0, 1.0, 1.0, m.alpha]);
                        pbr["baseColorTexture"] = json!({ "index": t });
                        cutout = alpha;
                    }
                    let mut gm = json!({ "name": mname, "pbrMetallicRoughness": pbr, "doubleSided": true });
                    if m.alpha < 0.99 {
                        gm["alphaMode"] = json!("BLEND");
                    } else if cutout {
                        gm["alphaMode"] = json!("MASK");
                        gm["alphaCutoff"] = json!(0.5);
                    }
                    gl_materials.push(gm);
                    mat_of.insert(mname.clone(), gl_materials.len() - 1);
                    gl_materials.len() - 1
                }
            };
            let pos: Vec<f32> = prim.pos.chunks(3).flat_map(|p| [-(p[0] + shift[0]), p[1] + shift[1], -(p[2] + shift[2])]).collect();
            let nrm: Vec<f32> = prim.nrm.chunks(3).flat_map(|n| [-n[0], n[1], -n[2]]).collect();
            let a_pos = buf.floats(&pos, 3, "VEC3", Some(34962), true);
            let a_nrm = buf.floats(&nrm, 3, "VEC3", Some(34962), false);
            let a_uv = buf.floats(&prim.uv, 2, "VEC2", Some(34962), false);
            let a_idx = buf.ints(&prim.idx, 34963);
            gl_prims.push(json!({ "attributes": { "POSITION": a_pos, "NORMAL": a_nrm, "TEXCOORD_0": a_uv }, "indices": a_idx, "material": material }));
        }
        if gl_prims.is_empty() {
            continue;
        }
        let n = used.entry(gname.clone()).or_insert(0usize);
        *n += 1;
        let name = if *n > 1 { format!("{gname} {n}") } else { gname.clone() };
        meshes.push(json!({ "name": name, "primitives": gl_prims }));
        nodes.push(json!({ "name": name, "mesh": meshes.len() - 1 }));
    }
    while buf.bin.len() % 4 != 0 {
        buf.bin.push(0);
    }
    let parts = nodes.len();
    let children: Vec<usize> = (1..=parts).collect();
    nodes.insert(0, json!({ "name": "Avatar", "children": children }));
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "rbxyoinker" },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": gl_materials,
        "buffers": [{ "byteLength": buf.bin.len() }],
        "bufferViews": buf.views,
        "accessors": buf.accessors,
    });
    if !gl_images.is_empty() {
        doc["images"] = json!(gl_images);
        doc["textures"] = json!(gl_textures);
        doc["samplers"] = json!([{ "magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497 }]);
    }
    let mut text = serde_json::to_vec(&doc).map_err(|e| e.to_string())?;
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + buf.bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&(buf.bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&buf.bin);
    Ok((out, parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdn_host_from_hash() {
        assert_eq!(cdn("https://tr.rbxcdn.com/x"), "https://tr.rbxcdn.com/x");
        assert!(cdn("abc").starts_with("https://t"));
    }

    #[test]
    fn builds_a_quad() {
        let obj = "g Head\nv 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nvn 0 0 1\nusemtl a\nf 1/1/1 2/2/1 3/3/1 4/4/1\n";
        let mats = parse_mtl("newmtl a\nKd 1 0 0\nd 1\n");
        let (glb, parts) = build(obj, &mats, &HashMap::new()).expect("glb");
        assert_eq!(parts, 1);
        assert_eq!(&glb[0..4], b"glTF");
        assert_eq!(u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize, glb.len());
    }

    #[test]
    fn resolves_links() {
        assert_eq!(number_after("https://www.roblox.com/users/156/profile", "/users/"), Some(156));
        assert_eq!(number_after("https://www.roblox.com/catalog/1374269/x", "/catalog/"), Some(1374269));
    }
}
