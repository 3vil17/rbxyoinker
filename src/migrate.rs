// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::convert;
use crate::download;
use crate::error::{Error, Result};
use crate::pool::Pool;
use crate::resolver;
use crate::roblox::{Creator, Roblox};
use crate::store;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Serialize, Clone, Default)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub type_label: String,
    pub owner: String,
    pub owner_name: String,
    pub state: String,
    pub new_id: Option<i64>,
    pub decal_id: Option<i64>,
    pub error: Option<String>,
    pub from_script: bool,
    pub script: String,
    pub line: String,
    pub reused: bool,
    #[serde(skip)]
    pub roblox_default: bool,
    pub reach: String,
    pub reach_error: Option<String>,
}

#[derive(Serialize, Default)]
pub struct Job {
    pub place: String,
    pub place_id: i64,
    pub checked: bool,
    pub running: bool,
    pub cancel: bool,
    pub run_id: u64,
    pub mode: String,
    pub target: String,
    pub note: String,
    pub scout: String,
    pub checking: bool,
    pub audio_quota: Option<(i64, i64)>,
    pub defaults_skipped: usize,
    pub items: Vec<Item>,
}

pub type Shared = Arc<Mutex<Job>>;

pub fn lock(job: &Shared) -> std::sync::MutexGuard<'_, Job> {
    job.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn kind_of(asset_type_id: i64) -> &'static str {
    match asset_type_id {
        3 => "Audio",
        1 | 13 => "Image",
        4 | 40 => "Mesh",
        24 => "Animation",
        _ => "",
    }
}

pub async fn check(rb: Roblox, pool: Pool, job: Shared) {
    let ids: Vec<(i64, bool)> = lock(&job).items.iter().map(|i| (i.id, i.from_script)).collect();
    let all: Vec<i64> = ids.iter().map(|(id, _)| *id).collect();
    if let Some(lease) = pool.acquire(&[]).await {
        rb.prefetch_details(&lease.cookie, &all).await;
    }
    let mut details = Vec::new();
    for (id, from_script) in &ids {
        let d = match rb.cached_details(*id) {
            Some(d) => Some(d),
            None if *from_script => None,
            None => rb.asset_details(*id).await.ok(),
        };
        details.push(d);
    }
    let mut owners: Vec<String> = details.iter().flatten().map(|d| format!("{}:{}", d.creator_type, d.creator_id)).collect();
    owners.sort();
    owners.dedup();
    let names = rb.owner_names(&owners).await;
    let mut j = lock(&job);
    for (it, d) in j.items.iter_mut().zip(details) {
        it.roblox_default = match &d {
            Some(d) => convert::is_roblox_default(it.id, d.asset_type_id, &d.creator_type, d.creator_id),
            None => convert::is_roblox_default(it.id, 0, "", 0),
        };
        if let Some(d) = d {
            it.name = d.name;
            it.kind = kind_of(d.asset_type_id).to_string();
            it.type_label = convert::type_label(d.asset_type_id).to_string();
            it.owner = format!("{}:{}", d.creator_type, d.creator_id);
            it.owner_name = names.get(&it.owner).cloned().unwrap_or(d.creator_name);
        }
    }
    j.items.retain(|it| !it.from_script || !it.kind.is_empty());
    if store::load_settings().skip_defaults {
        let before = j.items.len();
        j.items.retain(|it| !it.roblox_default);
        j.defaults_skipped = before - j.items.len();
    }
    j.checked = true;
}

fn upload_kind(kind: &str, bytes: &[u8]) -> Option<(&'static str, &'static str, &'static str)> {
    let ext = convert::extension_for(bytes, 0);
    Some(match (kind, ext) {
        ("Audio", "ogg") => ("Audio", "asset.ogg", "audio/ogg"),
        ("Audio", "mp3") => ("Audio", "asset.mp3", "audio/mpeg"),
        ("Audio", "wav") => ("Audio", "asset.wav", "audio/wav"),
        ("Image", "png") => ("Decal", "asset.png", "image/png"),
        ("Image", "jpg") => ("Decal", "asset.jpg", "image/jpeg"),
        ("Mesh", "mesh") => ("Mesh", "asset.mesh", "model/x-file-mesh-data"),
        ("Animation", "rbxm") => ("Animation", "asset.rbxm", "model/x-rbxm"),
        ("Animation", "rbxmx") => ("Animation", "asset.rbxmx", "model/x-rbxmx"),
        _ => return None,
    })
}

async fn image_inside(rb: &Roblox, cookie: &str, decal_id: i64) -> Option<i64> {
    for _ in 0..10 {
        if let Ok(bytes) = rb.fetch_cookie(cookie, decal_id, None, None).await {
            if let Some(inner) = convert::inner_asset_id(&bytes) {
                return Some(inner);
            }
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
    None
}

fn still_processing(decal: i64) -> Error {
    Error::Other(format!("uploaded as decal {decal}, Roblox is still processing it. Retry in a minute"))
}

async fn fetch(rb: &Roblox, pool: &Pool, place: Option<i64>, kind: &str, id: i64) -> Result<Vec<u8>> {
    let api_key = store::get_secret("api_key");
    let mut bytes = download::resolve_bytes(rb, api_key.as_deref(), pool, id, place).await?;
    if kind == "Image" {
        if let Some(inner) = convert::inner_asset_id(&bytes).filter(|i| *i != id) {
            bytes = download::resolve_bytes(rb, api_key.as_deref(), pool, inner, place).await?;
        }
        if convert::extension_for(&bytes, 1) == "webp" {
            bytes = convert::to_png(&bytes).ok_or_else(|| Error::Other("couldn't convert the webp image".into()))?;
        }
    }
    Ok(bytes)
}

async fn upload_one(rb: &Roblox, cookie: &str, to: &Creator, job: &Shared, i: usize, bytes: Option<Vec<u8>>, prefix: &str) -> Result<i64> {
    let item = lock(job).items[i].clone();
    if let Some(decal) = item.decal_id {
        return image_inside(rb, cookie, decal).await.ok_or_else(|| still_processing(decal));
    }
    let bytes = bytes.ok_or_else(|| Error::Other("nothing was downloaded".into()))?;
    let (asset_type, file_name, mime) = upload_kind(&item.kind, &bytes)
        .ok_or_else(|| Error::Other(format!("Roblox doesn't take this file as {}", item.kind.to_lowercase())))?;
    let base = if item.name.trim().is_empty() { format!("asset {}", item.id) } else { item.name.clone() };
    let name: String = format!("{prefix}{base}").chars().take(50).collect();
    let mut tries = 0;
    let new_id = loop {
        match rb.upload_asset(cookie, bytes.clone(), asset_type, &name, file_name, mime, to).await {
            Err(Error::RateLimited(s)) if tries < 3 => {
                tries += 1;
                tokio::time::sleep(Duration::from_secs(s.clamp(5, 60))).await;
            }
            other => break other?,
        }
    };
    if asset_type != "Decal" {
        return Ok(new_id);
    }
    lock(job).items[i].decal_id = Some(new_id);
    image_inside(rb, cookie, new_id).await.ok_or_else(|| still_processing(new_id))
}

fn set_result(job: &Shared, i: usize, res: Result<i64>) {
    let mut j = lock(job);
    let it = &mut j.items[i];
    match res {
        Ok(new_id) => {
            it.state = "ok".into();
            it.new_id = Some(new_id);
            it.error = None;
        }
        Err(e) => {
            it.state = "error".into();
            it.error = Some(e.to_string());
        }
    }
}

fn keep_copy(settings: &store::Settings, place: &str, item: &Item, bytes: &[u8]) -> std::io::Result<()> {
    let dir = std::path::Path::new(&settings.save_dir).join("Migrated").join(download::sanitize(place));
    std::fs::create_dir_all(&dir)?;
    let stem = download::file_stem(&settings.name_template, &item.name, item.id, &item.type_label, None);
    std::fs::write(dir.join(format!("{stem}.{}", convert::extension_for(bytes, 0))), bytes)
}

fn upload_gap(speed: &str) -> u64 {
    match speed {
        "safe" => 2000,
        "fast" => 250,
        _ => 800,
    }
}

async fn scout_for(rb: &Roblox, pool: &Pool, job: &Shared, todo: &[usize], place: Option<i64>) -> HashMap<i64, i64> {
    let items: Vec<(i64, String, i64)> = {
        let j = lock(job);
        todo.iter()
            .filter_map(|&i| {
                let it = &j.items[i];
                let (kind, id) = it.owner.split_once(':')?;
                Some((it.id, kind.to_string(), id.parse().ok()?))
            })
            .collect()
    };
    let (a, b) = (job.clone(), job.clone());
    let report = move |text: String| lock(&a).scout = text;
    let stop = move || lock(&b).cancel;
    resolver::scout(rb, pool, &items, place, &report, &stop).await
}

pub async fn dry_run(rb: Roblox, pool: Pool, job: Shared, cookie: String, to_key: String, mut todo: Vec<usize>) {
    let quota = rb.audio_quota(&cookie).await;
    lock(&job).audio_quota = quota;
    let settings = store::load_settings();
    if settings.migrate_reuse {
        if let Some(known) = store::load_moved().get(&to_key) {
            let mut j = lock(&job);
            todo.retain(|&i| {
                let it = &mut j.items[i];
                if !known.contains_key(&it.id.to_string()) {
                    return true;
                }
                it.reach = "reuse".into();
                false
            });
        }
    }
    let place = Some(lock(&job).place_id).filter(|p| *p > 0);
    let hints = Arc::new(scout_for(&rb, &pool, &job, &todo, place).await);
    let queue = Arc::new(todo);
    let next = Arc::new(AtomicUsize::new(0));
    let workers = (pool.alive_count() * 2).clamp(2, 8);
    let mut handles = Vec::new();
    for _ in 0..workers {
        let (rb, pool, job, queue, next, hints) = (rb.clone(), pool.clone(), job.clone(), queue.clone(), next.clone(), hints.clone());
        handles.push(tokio::spawn(async move {
            while let Some(&i) = queue.get(next.fetch_add(1, Ordering::SeqCst)) {
                let (id, kind) = {
                    let mut j = lock(&job);
                    if j.cancel {
                        j.items[i].reach.clear();
                        continue;
                    }
                    j.items[i].reach = "checking".into();
                    (j.items[i].id, j.items[i].kind.clone())
                };
                let hint = hints.get(&id).copied().or(place);
                let res = match fetch(&rb, &pool, hint, &kind, id).await {
                    Ok(bytes) if upload_kind(&kind, &bytes).is_some() => Ok(()),
                    Ok(_) => Err(format!("Roblox doesn't take this file as {}", kind.to_lowercase())),
                    Err(e) => Err(e.to_string()),
                };
                let mut j = lock(&job);
                let it = &mut j.items[i];
                it.reach = if res.is_ok() { "ok".into() } else { "no".into() };
                it.reach_error = res.err();
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
    let mut j = lock(&job);
    j.running = false;
    j.checking = false;
    j.cancel = false;
}

pub async fn run(rb: Roblox, pool: Pool, job: Shared, cookie: String, to: Creator, to_key: String, mut todo: Vec<usize>) {
    let settings = Arc::new(store::load_settings());
    let mut moved = store::load_moved();
    if settings.migrate_reuse {
        if let Some(known) = moved.get(&to_key) {
            let mut j = lock(&job);
            todo.retain(|&i| {
                let it = &mut j.items[i];
                let Some(&new_id) = known.get(&it.id.to_string()) else { return true };
                it.state = "ok".into();
                it.new_id = Some(new_id);
                it.reused = true;
                false
            });
        }
    }
    let (place, place_name) = {
        let j = lock(&job);
        (Some(j.place_id).filter(|p| *p > 0), j.place.clone())
    };
    let hints = Arc::new(scout_for(&rb, &pool, &job, &todo, place).await);
    let files: Arc<Mutex<HashMap<usize, Vec<u8>>>> = Arc::default();
    let queue = Arc::new(todo.clone());
    let next = Arc::new(AtomicUsize::new(0));
    let workers = (pool.alive_count() * 2).clamp(2, 8);
    let mut handles = Vec::new();
    for _ in 0..workers {
        let (rb, pool, job, files, queue, next) = (rb.clone(), pool.clone(), job.clone(), files.clone(), queue.clone(), next.clone());
        let (settings, place_name, hints) = (settings.clone(), place_name.clone(), hints.clone());
        handles.push(tokio::spawn(async move {
            while let Some(&i) = queue.get(next.fetch_add(1, Ordering::SeqCst)) {
                let item = {
                    let mut j = lock(&job);
                    if j.cancel {
                        break;
                    }
                    if j.items[i].decal_id.is_some() {
                        continue;
                    }
                    j.items[i].state = "downloading".into();
                    j.items[i].clone()
                };
                let hint = hints.get(&item.id).copied().or(place);
                match fetch(&rb, &pool, hint, &item.kind, item.id).await {
                    Ok(bytes) => {
                        if settings.migrate_keep {
                            if let Err(e) = keep_copy(&settings, &place_name, &item, &bytes) {
                                lock(&job).note = format!("Couldn't keep a copy: {e}");
                            }
                        }
                        files.lock().unwrap_or_else(|e| e.into_inner()).insert(i, bytes);
                        lock(&job).items[i].state = "downloaded".into();
                    }
                    Err(e) => set_result(&job, i, Err(e)),
                }
            }
        }));
    }
    for h in handles {
        let _ = h.await;
    }
    let gap = upload_gap(&settings.migrate_speed);
    let stop_after = settings.migrate_stop_after;
    let mut failed_in_a_row = 0;
    for i in todo {
        let bytes = {
            let mut j = lock(&job);
            if j.items[i].state == "error" {
                continue;
            }
            if j.cancel {
                j.items[i].state = "found".into();
                continue;
            }
            j.items[i].state = "uploading".into();
            files.lock().unwrap_or_else(|e| e.into_inner()).remove(&i)
        };
        let res = upload_one(&rb, &cookie, &to, &job, i, bytes, &settings.migrate_prefix).await;
        let new_id = res.as_ref().ok().copied();
        set_result(&job, i, res);
        if let Some(new_id) = new_id {
            failed_in_a_row = 0;
            let id = lock(&job).items[i].id;
            moved.entry(to_key.clone()).or_default().insert(id.to_string(), new_id);
            if let Err(e) = store::save_moved(&moved) {
                lock(&job).note = format!("Couldn't remember this move: {e}");
            }
        } else {
            failed_in_a_row += 1;
            if stop_after > 0 && failed_in_a_row >= stop_after {
                let mut j = lock(&job);
                j.cancel = true;
                j.note = format!("Stopped after {stop_after} failed uploads in a row.");
            }
        }
        tokio::time::sleep(Duration::from_millis(gap)).await;
    }
    let mut j = lock(&job);
    j.running = false;
    j.cancel = false;
}
