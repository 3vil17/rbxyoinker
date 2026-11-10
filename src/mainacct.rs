// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use crate::roblox::Roblox;
use serde::Serialize;
use serde_json::Value;

const MAIN_SCORE: u32 = 5;

#[derive(Default, Debug, Clone)]
pub struct Stats {
    pub age_days: Option<i64>,
    pub verified: bool,
    pub premium: bool,
    pub collectibles: usize,
    pub rap: i64,
    pub robux: Option<i64>,
    pub owned_groups: usize,
    pub high_rank_groups: usize,
    pub friends: Option<i64>,
    pub followers: Option<i64>,
    pub badges: usize,
    pub visits: i64,
}

#[derive(Serialize, Debug, Clone)]
pub struct Verdict {
    pub user_id: i64,
    pub looks_main: bool,
    pub score: u32,
    pub reasons: Vec<String>,
}

fn plural(n: impl Into<i64>, one: &str, many: &str) -> String {
    let n = n.into();
    format!("{n} {}", if n == 1 { one } else { many })
}

fn short(n: i64) -> String {
    match n {
        n if n >= 1_000_000 => format!("{:.1}M", n as f64 / 1_000_000.0),
        n if n >= 10_000 => format!("{}k", n / 1000),
        n => n.to_string(),
    }
}

pub fn score(s: &Stats) -> (u32, Vec<String>) {
    let mut points = 0;
    let mut why = Vec::new();
    let mut add = |p: u32, reason: String| {
        points += p;
        why.push(reason);
    };
    if let Some(days) = s.age_days {
        let years = days / 365;
        let p = match years {
            5.. => 3,
            2.. => 2,
            1.. => 1,
            _ => 0,
        };
        if p > 0 {
            add(p, format!("created {} ago", plural(years, "year", "years")));
        }
    }
    if s.verified {
        add(3, "has the verified badge".into());
    }
    if s.premium {
        add(2, "has Premium".into());
    }
    if s.collectibles > 0 {
        let p = match s.rap {
            10_000.. => 3,
            1_000.. => 2,
            _ => 1,
        };
        let worth = if s.rap > 0 { format!(" worth about {} R$", short(s.rap)) } else { String::new() };
        let count = if s.collectibles >= 100 { "100+ limiteds".to_string() } else { plural(s.collectibles as i64, "limited", "limiteds") };
        add(p, format!("owns {count}{worth}"));
    }
    if s.robux.is_some_and(|r| r >= 1_000) {
        add(1, format!("has {} Robux", short(s.robux.unwrap_or(0))));
    }
    if s.owned_groups > 0 {
        add(2, format!("owns {}", plural(s.owned_groups as i64, "group", "groups")));
    } else if s.high_rank_groups > 0 {
        add(1, format!("has a high rank in {}", plural(s.high_rank_groups as i64, "group", "groups")));
    }
    if s.friends.is_some_and(|f| f >= 100) {
        add(1, format!("{} friends", s.friends.unwrap_or(0)));
    }
    if s.followers.is_some_and(|f| f >= 100) {
        add(1, format!("{} followers", short(s.followers.unwrap_or(0))));
    }
    if s.badges >= 100 {
        add(1, "100+ badges".into());
    }
    if s.visits >= 1_000 {
        add(if s.visits >= 10_000 { 2 } else { 1 }, format!("games with {} visits", short(s.visits)));
    }
    (points, why)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn age_days(created: &str, now_unix: i64) -> Option<i64> {
    let date = created.get(..10)?;
    let mut parts = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (parts.next()??, parts.next()??, parts.next()??);
    Some((now_unix.div_euclid(86_400) - days_from_civil(y, m, d)).max(0))
}

fn count(v: Option<Value>) -> Option<i64> {
    v?.get("count")?.as_i64()
}

pub async fn check(rb: &Roblox, cookie: &str, user_id: i64) -> Verdict {
    let id = user_id;
    let url = [
        format!("https://users.roblox.com/v1/users/{id}"),
        format!("https://premiumfeatures.roblox.com/v1/users/{id}/validate-membership"),
        format!("https://inventory.roblox.com/v1/users/{id}/assets/collectibles?limit=100&sortOrder=Asc"),
        format!("https://economy.roblox.com/v1/users/{id}/currency"),
        format!("https://friends.roblox.com/v1/users/{id}/friends/count"),
        format!("https://friends.roblox.com/v1/users/{id}/followers/count"),
        format!("https://badges.roblox.com/v1/users/{id}/badges?limit=100&sortOrder=Desc"),
        format!("https://games.roblox.com/v2/users/{id}/games?accessFilter=Public&limit=50"),
    ];
    let (profile, premium, collectibles, currency, friends, followers, badges, games, groups) = tokio::join!(
        rb.get_json(&url[0], None),
        rb.get_json(&url[1], Some(cookie)),
        rb.get_json(&url[2], Some(cookie)),
        rb.get_json(&url[3], Some(cookie)),
        rb.get_json(&url[4], None),
        rb.get_json(&url[5], None),
        rb.get_json(&url[6], Some(cookie)),
        rb.get_json(&url[7], None),
        rb.user_groups(id),
    );
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    let items = collectibles.as_ref().and_then(|v| v["data"].as_array().cloned()).unwrap_or_default();
    let badge_list = badges.as_ref().and_then(|v| v["data"].as_array().map(|a| a.len())).unwrap_or(0);
    let more_badges = badges.as_ref().is_some_and(|v| v["nextPageCursor"].is_string());
    let stats = Stats {
        age_days: profile.as_ref().and_then(|p| p["created"].as_str()).and_then(|c| age_days(c, now)),
        verified: profile.as_ref().is_some_and(|p| p["hasVerifiedBadge"].as_bool() == Some(true)),
        premium: premium.is_some_and(|p| p.as_bool() == Some(true)),
        collectibles: items.len() + if collectibles.as_ref().is_some_and(|v| v["nextPageCursor"].is_string()) { 100 } else { 0 },
        rap: items.iter().filter_map(|i| i["recentAveragePrice"].as_i64()).sum(),
        robux: currency.and_then(|c| c["robux"].as_i64()),
        owned_groups: groups.iter().filter(|g| g.rank >= 255).count(),
        high_rank_groups: groups.iter().filter(|g| (200..255).contains(&g.rank)).count(),
        friends: count(friends),
        followers: count(followers),
        badges: if more_badges { 100 } else { badge_list },
        visits: games
            .as_ref()
            .and_then(|g| g["data"].as_array().map(|a| a.iter().filter_map(|x| x["placeVisits"].as_i64()).sum()))
            .unwrap_or(0),
    };
    let (score, reasons) = score(&stats);
    Verdict { user_id, looks_main: score >= MAIN_SCORE, score, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_alt_is_not_main() {
        let s = Stats { age_days: Some(40), friends: Some(3), ..Default::default() };
        let (points, why) = score(&s);
        assert!(points < MAIN_SCORE);
        assert!(why.is_empty());
    }

    #[test]
    fn old_empty_alt_is_not_main() {
        let s = Stats { age_days: Some(365 * 6), ..Default::default() };
        assert!(score(&s).0 < MAIN_SCORE);
    }

    #[test]
    fn old_account_with_limiteds_and_groups_is_main() {
        let s = Stats {
            age_days: Some(365 * 15),
            collectibles: 12,
            rap: 45_000,
            owned_groups: 2,
            friends: Some(180),
            ..Default::default()
        };
        let (points, why) = score(&s);
        assert!(points >= MAIN_SCORE);
        assert_eq!(why[0], "created 15 years ago");
        assert_eq!(why[1], "owns 12 limiteds worth about 45k R$");
        assert_eq!(why[2], "owns 2 groups");
    }

    #[test]
    fn verified_premium_is_main() {
        let s = Stats { verified: true, premium: true, ..Default::default() };
        assert!(score(&s).0 >= MAIN_SCORE);
    }

    #[test]
    fn age_from_roblox_dates() {
        let now = days_from_civil(2026, 11, 10) * 86_400 + 3600;
        assert_eq!(age_days("2011-07-30T12:00:00.000Z", now), Some(days_from_civil(2026, 11, 10) - days_from_civil(2011, 7, 30)));
        assert_eq!(age_days("2026-11-10T00:00:00Z", now), Some(0));
        assert_eq!(age_days("nonsense", now), None);
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }
}
