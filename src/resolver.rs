// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::error::{Error, Result};
use crate::pool::Pool;
use crate::roblox::{PlaceRef, Roblox};
use std::collections::HashMap;

pub struct Attempt {
    pub bytes: Vec<u8>,
    pub method: String,
    pub place_id: Option<i64>,
    pub account: Option<String>,
}

struct Denial {
    error: Error,
    cookie: String,
    label: String,
}

async fn via_cookie(
    rb: &Roblox,
    pool: &Pool,
    id: i64,
    place: Option<i64>,
    version: Option<u32>,
    exclude: &[String],
) -> std::result::Result<(Vec<u8>, String), Option<Denial>> {
    for _ in 0..8 {
        let lease = pool.acquire(exclude).await.ok_or(None)?;
        match rb.fetch_cookie(&lease.cookie, id, place, version).await {
            Ok(bytes) => {
                pool.report_ok(&lease.cookie);
                return Ok((bytes, lease.label));
            }
            Err(Error::RateLimited(s)) => pool.report_limited(&lease.cookie, s),
            Err(e) => {
                if e.to_string().starts_with("401") {
                    pool.report_dead(&lease.cookie);
                } else {
                    pool.report_ok(&lease.cookie);
                }
                return Err(Some(Denial { error: e, cookie: lease.cookie, label: lease.label }));
            }
        }
    }
    Err(None)
}

pub async fn resolve(
    rb: &Roblox,
    api_key: Option<&str>,
    pool: &Pool,
    id: i64,
    version: Option<u32>,
    creator: Option<(String, i64)>,
    hint_place: Option<i64>,
) -> Result<Attempt> {
    let has_cookies = pool.alive_count() > 0;
    if api_key.is_none() && !has_cookies {
        if pool.total_count() > 0 {
            return Err(Error::Denied(
                "every saved cookie is expired or invalid, replace them in Settings and Recheck in Accounts".into(),
            ));
        }
        return Err(Error::NoCredentials);
    }
    let mut notes: Vec<String> = Vec::new();
    let mut limited = false;

    if let Some(key) = api_key {
        match rb.fetch_opencloud(key, id, version).await {
            Ok(bytes) => {
                return Ok(Attempt { bytes, method: "Open Cloud API key".into(), place_id: None, account: None })
            }
            Err(Error::RateLimited(_)) => {
                limited = true;
                notes.push("Open Cloud: rate limited".into());
            }
            Err(e) => notes.push(format!("Open Cloud: {e}")),
        }
    }

    if !has_cookies {
        notes.push("Cookie: none saved or all expired".into());
        return Err(finish(notes, limited));
    }

    let mut tried: Vec<String> = Vec::new();
    loop {
        match via_cookie(rb, pool, id, None, version, &tried).await {
            Ok((bytes, label)) => {
                return Ok(Attempt {
                    bytes,
                    method: format!("Cookie (direct) \u{b7} {label}"),
                    place_id: None,
                    account: Some(label),
                })
            }
            Err(Some(d)) => {
                notes.push(format!("Cookie {}: {}", d.label, d.error));
                tried.push(d.cookie);
            }
            Err(None) => {
                if tried.is_empty() {
                    limited = true;
                    notes.push("Cookie: every account is rate limited".into());
                }
                break;
            }
        }
    }

    let hint_place = hint_place.filter(|p| *p > 0);
    if let Some(pid) = hint_place {
        match via_cookie(rb, pool, id, Some(pid), version, &[]).await {
            Ok((bytes, label)) => {
                return Ok(Attempt {
                    bytes,
                    method: format!("Cookie \u{b7} {label} \u{b7} Place-Id {pid} (the place it's used in)"),
                    place_id: Some(pid),
                    account: Some(label),
                })
            }
            Err(Some(d)) => notes.push(format!("Place-Id {pid} (the place it's used in): {}", d.error)),
            Err(None) => {
                limited = true;
                notes.push("Place-Id (the place it's used in): rate limited".into());
            }
        }
    }

    if rb.was_scouted(id) {
        notes.push("Place search: already checked against every game we could find".into());
        return Err(finish(notes, limited));
    }

    let Some((ctype, cid)) = creator.filter(|(_, cid)| *cid > 0) else {
        notes.push("Place-Id fallback: skipped, asset has no public creator info".into());
        return Err(finish(notes, limited));
    };
    let mut tried_places: Vec<i64> = hint_place.into_iter().collect();
    let owner = (ctype.as_str(), cid);
    let known = rb.known_places(&ctype, cid);
    if let Places::Hit(a) = try_places(rb, pool, id, version, &known, None, owner, &mut tried_places).await {
        return Ok(a);
    }
    let places = rb.list_creator_places(&ctype, cid, 60).await;
    if places.is_empty() {
        notes.push(format!("Place-Id fallback: the creator ({ctype} {cid}) has no public places"));
    } else {
        match try_places(rb, pool, id, version, &places, None, owner, &mut tried_places).await {
            Places::Hit(a) => return Ok(a),
            Places::Limited(n) => {
                notes.push(format!("Place-Id fallback: rate limited after {n} places"));
                return Err(finish(notes, true));
            }
            Places::Denied(n, last) => notes.push(format!(
                "Place-Id fallback: {n} public places from the creator, all denied (last: {last})"
            )),
        }
    }

    let is_user = ctype.eq_ignore_ascii_case("User");
    if rb.search_spent(&ctype, cid) {
        notes.push("Wider search: skipped, it already came up empty for this creator a few times".into());
        return Err(finish(notes, limited));
    }
    let person = if is_user { Some(cid) } else { rb.group_owner(cid).await };
    let Some(person) = person else {
        notes.push("Wider search: the group has no owner".into());
        return Err(finish(notes, limited));
    };
    if !is_user {
        let own = rb.list_creator_places("User", person, 60).await;
        match try_places(rb, pool, id, version, &own, Some("the group owner"), owner, &mut tried_places).await {
            Places::Hit(a) => return Ok(a),
            Places::Limited(n) => {
                notes.push(format!("Group owner's games: rate limited after {n} places"));
                return Err(finish(notes, true));
            }
            Places::Denied(n, _) if n > 0 => notes.push(format!("Group owner's games: {n} places, all denied")),
            Places::Denied(..) => {}
        }
    }

    let skip = if is_user { None } else { Some(cid) };
    let linked = linked_places(rb, person, skip).await;
    let who = if is_user { "the creator" } else { "the group owner" };
    if linked.is_empty() {
        notes.push(format!("Group games: {who} is in no public groups with games"));
        return Err(finish(notes, limited));
    }
    let mut count = 0;
    let mut last = String::new();
    for (place, group) in &linked {
        match try_places(rb, pool, id, version, std::slice::from_ref(place), Some(group), owner, &mut tried_places).await {
            Places::Hit(a) => return Ok(a),
            Places::Limited(n) => {
                notes.push(format!("Group games: rate limited after {} places", count + n));
                return Err(finish(notes, true));
            }
            Places::Denied(n, l) => {
                count += n;
                if !l.is_empty() {
                    last = l;
                }
            }
        }
    }
    rb.note_search_miss(&ctype, cid);
    notes.push(format!(
        "Group games: {count} places from groups {who} is in, all denied{}",
        if last.is_empty() { String::new() } else { format!(" (last: {last})") }
    ));
    Err(finish(notes, limited))
}

const STAFF_GAMES: usize = 10;
const MEMBER_GAMES: usize = 3;
const LINKED_LIMIT: usize = 150;

async fn linked_places(rb: &Roblox, person: i64, skip: Option<i64>) -> Vec<(PlaceRef, String)> {
    let groups: Vec<_> = rb.user_groups(person).await.into_iter().filter(|g| Some(g.id) != skip).collect();
    let mut staff = Vec::new();
    let mut members = Vec::new();
    for chunk in groups.chunks(6) {
        let mut set = tokio::task::JoinSet::new();
        for g in chunk {
            let (rb, g) = (rb.clone(), g.clone());
            set.spawn(async move {
                let places = rb.list_creator_places("Group", g.id, STAFF_GAMES).await;
                (g, places)
            });
        }
        while let Some(Ok((g, places))) = set.join_next().await {
            let (list, keep) = if g.rank > 1 { (&mut staff, STAFF_GAMES) } else { (&mut members, MEMBER_GAMES) };
            list.extend(places.into_iter().take(keep).map(|p| (p, format!("group {}", g.name))));
        }
    }
    staff.sort_by(|a: &(PlaceRef, String), b| b.0.visits.cmp(&a.0.visits));
    members.sort_by(|a: &(PlaceRef, String), b| b.0.visits.cmp(&a.0.visits));
    staff.extend(members);
    staff.truncate(LINKED_LIMIT);
    staff
}

const SCOUT_CHUNK: usize = 100;

async fn reachable(rb: &Roblox, pool: &Pool, ids: &[i64], place: Option<i64>) -> Option<Vec<i64>> {
    let mut out = Vec::new();
    for chunk in ids.chunks(SCOUT_CHUNK) {
        let mut done = false;
        for _ in 0..4 {
            let lease = pool.acquire(&[]).await?;
            match rb.batch_reachable(&lease.cookie, chunk, place).await {
                Ok(hits) => {
                    pool.report_ok(&lease.cookie);
                    out.extend(hits);
                    done = true;
                    break;
                }
                Err(Error::RateLimited(s)) => pool.report_limited(&lease.cookie, s),
                Err(_) => {
                    pool.report_ok(&lease.cookie);
                    return None;
                }
            }
        }
        if !done {
            return None;
        }
    }
    Some(out)
}

async fn all_places(rb: &Roblox, ctype: &str, cid: i64) -> Vec<PlaceRef> {
    let mut out = rb.known_places(ctype, cid);
    out.extend(rb.list_creator_places(ctype, cid, 60).await);
    let is_user = ctype.eq_ignore_ascii_case("User");
    let person = if is_user { Some(cid) } else { rb.group_owner(cid).await };
    if let Some(person) = person {
        if !is_user {
            out.extend(rb.list_creator_places("User", person, 60).await);
        }
        let skip = if is_user { None } else { Some(cid) };
        out.extend(linked_places(rb, person, skip).await.into_iter().map(|(p, _)| p));
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|p| seen.insert(p.place_id));
    out
}

pub async fn scout(
    rb: &Roblox,
    pool: &Pool,
    items: &[(i64, String, i64)],
    first: Option<i64>,
    report: &(dyn Fn(String) + Send + Sync),
    stop: &(dyn Fn() -> bool + Send + Sync),
) -> HashMap<i64, i64> {
    let mut hints = HashMap::new();
    let ids: Vec<i64> = items.iter().map(|(id, _, _)| *id).collect();
    ids.iter().for_each(|id| rb.clear_scouted(*id));
    report("Checking what needs a game".into());
    let Some(direct) = reachable(rb, pool, &ids, None).await else {
        report(String::new());
        return hints;
    };
    let mut left: Vec<&(i64, String, i64)> = items.iter().filter(|(id, _, _)| !direct.contains(id)).collect();
    if let Some(place) = first.filter(|p| *p > 0) {
        let ids: Vec<i64> = left.iter().map(|it| it.0).collect();
        if let Some(hits) = reachable(rb, pool, &ids, Some(place)).await {
            hits.iter().for_each(|id| {
                hints.insert(*id, place);
            });
            left.retain(|it| !hits.contains(&it.0));
        }
    }
    let mut creators: HashMap<(String, i64), Vec<i64>> = HashMap::new();
    for (id, ctype, cid) in left {
        creators.entry((ctype.clone(), *cid)).or_default().push(*id);
    }
    for ((ctype, cid), mut ids) in creators {
        if ids.len() < 2 || cid <= 0 {
            continue;
        }
        let places = all_places(rb, &ctype, cid).await;
        let mut finished = true;
        for (n, p) in places.iter().enumerate() {
            if ids.is_empty() {
                break;
            }
            if stop() {
                finished = false;
                break;
            }
            report(format!("Checking {} ({} of {} games)", p.name, n + 1, places.len()));
            let Some(hits) = reachable(rb, pool, &ids, Some(p.place_id)).await else {
                finished = false;
                break;
            };
            if !hits.is_empty() {
                rb.remember_place(&ctype, cid, p);
                hits.iter().for_each(|id| {
                    hints.insert(*id, p.place_id);
                });
                ids.retain(|id| !hits.contains(id));
            }
        }
        if finished {
            ids.iter().for_each(|id| rb.mark_scouted(*id));
        }
    }
    report(String::new());
    hints
}

enum Places {
    Hit(Attempt),
    Denied(usize, String),
    Limited(usize),
}

async fn try_places(
    rb: &Roblox,
    pool: &Pool,
    id: i64,
    version: Option<u32>,
    places: &[PlaceRef],
    group: Option<&str>,
    owner: (&str, i64),
    tried: &mut Vec<i64>,
) -> Places {
    let mut count = 0;
    let mut last = String::new();
    for p in places {
        if tried.contains(&p.place_id) {
            continue;
        }
        tried.push(p.place_id);
        rb.set_progress(id, format!("Trying {} ({} games tried)", p.name, tried.len()));
        match via_cookie(rb, pool, id, Some(p.place_id), version, &[]).await {
            Ok((bytes, label)) => {
                rb.remember_place(owner.0, owner.1, p);
                let via = match group {
                    Some(g) => format!(" via {g}"),
                    None => String::new(),
                };
                return Places::Hit(Attempt {
                    bytes,
                    method: format!("Cookie \u{b7} {label} \u{b7} Place-Id {} ({}){via}", p.place_id, p.name),
                    place_id: Some(p.place_id),
                    account: Some(label),
                });
            }
            Err(Some(d)) => {
                count += 1;
                last = d.error.to_string();
            }
            Err(None) => return Places::Limited(count),
        }
    }
    Places::Denied(count, last)
}

fn finish(notes: Vec<String>, limited: bool) -> Error {
    if limited && notes.iter().all(|n| n.contains("rate limited")) {
        Error::RateLimited(0)
    } else {
        Error::Denied(notes.join(" | "))
    }
}
