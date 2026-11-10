// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::convert;
use crate::error::{Error, Result};
use crate::resolver;
use crate::pool::Pool;
use crate::roblox::Roblox;
use crate::store;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct DownloadInfo {
    pub id: i64,
    pub name: String,
    pub type_label: String,
    pub method: String,
    pub place_id: Option<i64>,
    pub account: Option<String>,
    pub path: String,
}

fn now_millis() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_default()
}

fn clean(text: &str, max: usize) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) || c.is_control() { '_' } else { c })
        .collect();
    cleaned.trim().trim_matches('.').trim().chars().take(max).collect()
}

pub fn sanitize(name: &str) -> String {
    let out = clean(name, 80);
    if out.is_empty() { "asset".into() } else { out }
}

pub fn file_stem(template: &str, name: &str, id: i64, type_label: &str, version: Option<u32>) -> String {
    let t = if template.trim().is_empty() { store::DEFAULT_TEMPLATE } else { template };
    let mut out = t
        .replace("{name}", &sanitize(name))
        .replace("{id}", &id.to_string())
        .replace("{type}", type_label)
        .replace("{version}", &version.map(|v| v.to_string()).unwrap_or_default());
    if let Some(v) = version.filter(|_| !t.contains("{version}")) {
        out.push_str(&format!("_v{v}"));
    }
    let out = clean(&out, 150);
    if out.is_empty() { format!("asset_{id}") } else { out }
}

async fn explain(rb: &Roblox, id: i64, err: Error) -> Error {
    let Error::Denied(msg) = err else { return err };
    let (_, state) = rb.thumbnail(id).await;
    match state.as_deref() {
        Some("InReview") | Some("Pending") => Error::Denied(format!("{msg} | Roblox status: this asset is in review")),
        Some("Blocked") => Error::Denied(format!("{msg} | Roblox status: this asset was moderated")),
        _ => Error::Denied(msg),
    }
}

const HEADS: [i64; 2] = [17, 79];

fn write_mesh_extras(path: &std::path::Path, bytes: &[u8], settings: &store::Settings) {
    if !settings.mesh_obj && !settings.mesh_glb {
        return;
    }
    let Ok(mesh) = rbxmesh::read(bytes) else { return };
    if settings.mesh_obj {
        if let Ok(obj) = rbxmesh::to_obj(&mesh, 0) {
            let _ = std::fs::write(path.with_extension("obj"), obj);
        }
    }
    if settings.mesh_glb {
        if let Ok(glb) = rbxmesh::to_glb(&mesh, 0) {
            let _ = std::fs::write(path.with_extension("glb"), glb);
        }
    }
}

async fn save_dynamic_head(
    rb: &Roblox,
    api_key: Option<&str>,
    pool: &Pool,
    id: i64,
    hint: Option<i64>,
    full: &std::path::Path,
    settings: &store::Settings,
) -> bool {
    let Some(cookie) = pool.alive_users().into_iter().find_map(|(uid, _, _)| pool.cookie_for_user(uid)) else {
        return false;
    };
    let Some(model) = rb.fetch_format(&cookie, id, "avatar_meshpart_head").await else { return false };
    if !model.starts_with(b"<roblox") {
        return false;
    }
    let (Some(dir), Some(stem)) = (full.parent(), full.file_stem().map(|s| s.to_string_lossy().to_string())) else {
        return false;
    };
    if std::fs::write(dir.join(format!("{stem}_Animated.rbxm")), &model).is_err() {
        return false;
    }
    for inner in crate::scan::asset_ids(&model).into_iter().filter(|&i| i != id) {
        let Ok(found) = resolver::resolve(rb, api_key, pool, inner, None, None, hint).await else { continue };
        if rbxmesh::is_mesh(&found.bytes) {
            let mesh_path = dir.join(format!("{stem}_Animated.mesh"));
            if std::fs::write(&mesh_path, &found.bytes).is_ok() {
                write_mesh_extras(&mesh_path, &found.bytes, settings);
            }
            break;
        }
    }
    true
}

pub async fn fetch_and_save(
    rb: &Roblox,
    api_key: Option<&str>,
    pool: &Pool,
    id: i64,
    version: Option<u32>,
    hint: Option<i64>,
    settings: &store::Settings,
) -> Result<DownloadInfo> {
    let details = rb.asset_details(id).await.ok();
    let (name, atype0, creator) = match &details {
        Some(d) => (d.name.clone(), d.asset_type_id, Some((d.creator_type.clone(), d.creator_id))),
        None => (format!("asset_{id}"), 0, None),
    };

    if name == "[ Content Deleted ]" {
        return Err(Error::Denied("this asset was moderated by Roblox (content deleted)".into()));
    }
    let mut attempt = match resolver::resolve(rb, api_key, pool, id, version, creator, hint).await {
        Ok(a) => a,
        Err(Error::Denied(msg)) if atype0 == 10 => {
            return Err(Error::Denied(format!(
                "{msg} | Note: this is a model. Packages and private models only download for accounts that have access to them"
            )))
        }
        Err(e) => return Err(explain(rb, id, e).await),
    };

    let mut atype = atype0;
    let follow = matches!(atype0, 0 | 1 | 13);
    if let Some(inner) = convert::inner_asset_id(&attempt.bytes).filter(|_| follow) {
        if inner != id {
            if let Ok(a2) = resolver::resolve(rb, api_key, pool, inner, None, None, hint).await {
                attempt.bytes = a2.bytes;
                attempt.method = format!("{} (decal \u{2192} {})", attempt.method, inner);
                atype = 1;
            }
        }
    }

    let mut ext = convert::extension_for(&attempt.bytes, atype);
    if settings.images_png && matches!(ext, "jpg" | "webp") {
        if let Some(png) = convert::to_png(&attempt.bytes) {
            attempt.bytes = png;
            ext = "png";
        }
    }
    let type_label = convert::type_label(atype0).to_string();
    let filename = format!("{}.{}", file_stem(&settings.name_template, &name, id, &type_label, version), ext);

    let mut dir = std::path::PathBuf::from(&settings.save_dir);
    if settings.sort_by_type {
        dir.push(sanitize(&type_label));
    }
    std::fs::create_dir_all(&dir)?;
    let full = dir.join(&filename);
    std::fs::write(&full, &attempt.bytes)?;
    if ext == "mesh" {
        write_mesh_extras(&full, &attempt.bytes, settings);
    }
    if HEADS.contains(&atype0) && version.is_none() && save_dynamic_head(rb, api_key, pool, id, hint, &full, settings).await {
        attempt.method.push_str(" + animated head");
    }
    let path = full.to_string_lossy().to_string();

    let _ = store::push_history(store::HistoryEntry {
        id,
        name: name.clone(),
        type_label: type_label.clone(),
        path: path.clone(),
        method: attempt.method.clone(),
        when: now_millis(),
    });

    Ok(DownloadInfo {
        id,
        name,
        type_label,
        method: attempt.method,
        place_id: attempt.place_id,
        account: attempt.account,
        path,
    })
}

pub async fn resolve_bytes(
    rb: &Roblox,
    api_key: Option<&str>,
    pool: &Pool,
    id: i64,
    place: Option<i64>,
) -> Result<Vec<u8>> {
    let creator = rb.asset_details(id).await.ok().map(|d| (d.creator_type, d.creator_id));
    match resolver::resolve(rb, api_key, pool, id, None, creator, place).await {
        Ok(a) => Ok(a.bytes),
        Err(e) => Err(explain(rb, id, e).await),
    }
}
