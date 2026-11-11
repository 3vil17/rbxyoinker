// Copyright (C) 2026 3vil17. AGPL-3.0 with additional terms, see LICENSE and NOTICE.md.
use std::time::{Duration, Instant};
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

const LOGIN_URL: &str = "https://www.roblox.com/login";
pub const COOKIE_NAME: &str = ".ROBLOSECURITY";
pub const DONE_PREFIX: &str = "rbxyoinker-login:";

const ALLOWED: [&str; 4] = ["roblox.com", "rbxcdn.com", "arkoselabs.com", "funcaptcha.com"];

fn page_allowed(url: &str) -> bool {
    if url.starts_with("about:") || url.starts_with("blob:") || url.starts_with("data:") {
        return true;
    }
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("").rsplit('@').next().unwrap_or("").split(':').next().unwrap_or("");
    ALLOWED.iter().any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

fn data_dir() -> std::path::PathBuf {
    std::env::temp_dir().join("rbxyoinker-login")
}

pub fn run() -> i32 {
    let dir = data_dir();
    let _ = std::fs::remove_dir_all(&dir);
    #[cfg(windows)]
    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", &dir);
    let event_loop = EventLoop::new();
    let window = match WindowBuilder::new()
        .with_title("Log in to Roblox - rbxyoinker")
        .with_inner_size(tao::dpi::LogicalSize::new(500.0, 760.0))
        .build(&event_loop)
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("couldn't open the login window: {e}");
            return 3;
        }
    };
    let webview = match WebViewBuilder::new()
        .with_url(LOGIN_URL)
        .with_incognito(true)
        .with_navigation_handler(|url| page_allowed(&url))
        .with_new_window_req_handler(|_, _| wry::NewWindowResponse::Deny)
        .build(&window)
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("couldn't start the login page: {e}");
            return 4;
        }
    };
    let started = Instant::now();
    event_loop.run(move |event, _, flow| {
        *flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(500));
        match event {
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let found = webview
                    .cookies_for_url("https://www.roblox.com")
                    .unwrap_or_default()
                    .into_iter()
                    .find(|c| c.name() == COOKIE_NAME && c.value().starts_with("_|WARNING"));
                if let Some(c) = found {
                    println!("{DONE_PREFIX}{}", c.value());
                    finish(0);
                }
                if started.elapsed() > Duration::from_secs(15 * 60) {
                    finish(5);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => finish(2),
            _ => {}
        }
    });
}

fn finish(code: i32) -> ! {
    let _ = std::fs::remove_dir_all(data_dir());
    std::process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::page_allowed;

    #[test]
    fn only_roblox_pages_load() {
        assert!(page_allowed("https://www.roblox.com/login"));
        assert!(page_allowed("https://roblox.com/home"));
        assert!(page_allowed("https://auth.roblox.com/v2/login?x=1"));
        assert!(!page_allowed("https://roblox.com.evil.example/login"));
        assert!(!page_allowed("https://evilroblox.com/"));
        assert!(!page_allowed("http://www.roblox.com/login"));
        assert!(!page_allowed("https://example.com/?u=roblox.com"));
        assert!(!page_allowed("https://roblox.com@evil.example/"));
        assert!(page_allowed("https://client-api.arkoselabs.com/fc/gc/"));
    }
}
