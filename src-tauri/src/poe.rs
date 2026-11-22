//! The web requests the page may not make itself (poe.ninja prices, the
//! pathofexile.com league list), and the pathofexile.com sign-in (in a window
//! of its own; the account name is read from the player's account page there).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub const MAIN_LABEL: &str = "main";
pub const LOGIN_LABEL: &str = "poe-login";
/// The same as the main window in tauri.conf.json ("additionalBrowserArgs"):
/// WebView2 only opens more windows on the same data folder with the very
/// same settings, or it fails with 0x8007139F.
pub(crate) const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows";
const POE_SITE: &str = "https://www.pathofexile.com/";
/// Where a window's answer comes back (never a real site).
pub(crate) const RETURN_HOST: &str = "vaalstreet.invalid";

/// Hosts the page may reach through `vs_fetch`.
const PROXY_HOSTS: [&str; 2] = ["www.pathofexile.com", "poe.ninja"];
/// Shortest wait between two downloads of the same cached list.
const CACHE_MIN_AGE: u64 = 6 * 60 * 60;
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36 VaalStreet/1.0";

pub struct Poe {
    http: reqwest::Client,
    jar: std::sync::Arc<reqwest::cookie::Jar>,
    signed_in: AtomicBool,
}

impl Poe {
    pub fn new() -> Self {
        let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
        let http = reqwest::Client::builder()
            .cookie_provider(jar.clone())
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(30))
            .build()
            .expect("http client");
        Poe { http, jar, signed_in: AtomicBool::new(false) }
    }
}

fn poe(app: &AppHandle) -> &Poe {
    app.state::<Poe>().inner()
}

pub(crate) fn poe_signed_in(app: &AppHandle) -> bool {
    poe(app).signed_in.load(Ordering::SeqCst)
}

/// `%APPDATA%\VaalStreet`: settings, the wealth file, the cache, the log.
pub(crate) fn data_dir(_app: &AppHandle) -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("VaalStreet")
}

pub(crate) fn send(app: &AppHandle, name: &str, payload: Value) {
    let _ = app.emit("vs-event", json!({ "name": name, "payload": payload }));
}

/// A line in `%APPDATA%\VaalStreet\vaalstreet.log` (never the session itself).
pub(crate) fn trace(app: &AppHandle, message: &str) {
    use std::io::Write;
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let line = format!("[{:02}:{:02}:{:02} UTC] {message}\n", secs / 3600 % 24, secs / 60 % 60, secs % 60);
    let dir = data_dir(app);
    let path = dir.join("vaalstreet.log");
    if fs::metadata(&path).is_ok_and(|m| m.len() > 512 * 1024) {
        let _ = fs::rename(&path, dir.join("vaalstreet.old.log"));
    }
    let _ = fs::create_dir_all(&dir);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// Opens a web link in the player's own browser.
#[tauri::command]
pub fn vs_open_url(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("only web links open".into());
    }
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", parsed.as_str()])
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

// ------------------------------------------------------------------ fetches

#[derive(serde::Serialize, serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Fetched {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: String,
}

#[tauri::command]
pub async fn vs_fetch(app: AppHandle, url: String) -> Result<Fetched, String> {
    let parsed = reqwest::Url::parse(&url).map_err(|e| e.to_string())?;
    let host = parsed.host_str().unwrap_or("").to_string();
    if parsed.scheme() != "https" || !PROXY_HOSTS.contains(&host.as_str()) {
        return Err(format!("{host} is not allowed"));
    }
    let response = poe(&app).http.get(parsed).send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .filter(|(name, _)| !matches!(name.as_str(), "content-encoding" | "content-length" | "transfer-encoding"))
        .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
        .collect();
    let body = response.text().await.map_err(|e| e.to_string())?;
    Ok(Fetched { status: status.as_u16(), status_text: status.canonical_reason().unwrap_or("").to_string(), headers, body })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    saved: u64,
    fetched: Fetched,
}

fn cache_path(app: &AppHandle, url: &str) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    data_dir(app).join("cache").join(format!("{:016x}.json", hasher.finish()))
}

/// A data list fetched at most once per `max_age_secs` (never more often than
/// every 6 hours) and kept on disk; the last good copy when a site is down.
#[tauri::command]
pub async fn vs_cached(app: AppHandle, url: String, max_age_secs: u64) -> Result<Fetched, String> {
    let path = cache_path(&app, &url);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let stored: Option<CacheEntry> = fs::read(&path).ok().and_then(|bytes| serde_json::from_slice(&bytes).ok());
    if let Some(entry) = &stored {
        if now.saturating_sub(entry.saved) < max_age_secs.max(CACHE_MIN_AGE) {
            return Ok(entry.fetched.clone());
        }
    }
    match vs_fetch(app.clone(), url.clone()).await {
        Ok(fetched) if (200..300).contains(&fetched.status) => {
            let entry = CacheEntry { saved: now, fetched };
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            if let Ok(bytes) = serde_json::to_vec(&entry) {
                let tmp = path.with_extension("partial");
                if fs::write(&tmp, bytes).is_ok() {
                    let _ = fs::rename(&tmp, &path);
                }
            }
            Ok(entry.fetched)
        }
        other => {
            if let Some(entry) = stored {
                return Ok(entry.fetched);
            }
            match other {
                Ok(fetched) => Err(format!("{} answered {}", url, fetched.status)),
                Err(error) => Err(error),
            }
        }
    }
}

// ------------------------------------------------------------------ sign-in

/// Opens pathofexile.com in a window of its own (its account page, which sends
/// the player to the sign-in form when needed). Vaal Street never sees the
/// password. Once signed in, the account name is read there.
#[tauri::command]
pub async fn vs_login(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(LOGIN_LABEL) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }
    trace(&app, "sign-in asked for (pathofexile.com)");
    let url = format!("{POE_SITE}my-account").parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let nav_app = app.clone();
    let built = tauri::WebviewWindowBuilder::new(&app, LOGIN_LABEL, tauri::WebviewUrl::External(url))
        .additional_browser_args(BROWSER_ARGS)
        .title("Sign in to pathofexile.com — Vaal Street")
        .inner_size(560.0, 780.0)
        .center()
        // Vaal Street's own window stays over the game: this one goes over it
        .always_on_top(true)
        .focused(true)
        .on_navigation(move |url| {
            if url.host_str() != Some(RETURN_HOST) {
                return true;
            }
            if url.path() == "/account" {
                let name = url.fragment().map(percent_decode).unwrap_or_default();
                if !name.trim().is_empty() {
                    crate::wealth::set_account(&nav_app, name.trim());
                }
            }
            false
        })
        .build();
    if let Err(error) = &built {
        trace(&app, &format!("could not open the sign-in window: {error}"));
    }
    built.map_err(|e| e.to_string())?;
    let app = app.clone();
    thread::spawn(move || watch_login(&app));
    Ok(())
}

/// Signs out: the session is forgotten and its cookie deleted.
#[tauri::command]
pub async fn vs_logout(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        if let Ok(url) = POE_SITE.parse::<tauri::Url>() {
            for cookie in win.cookies_for_url(url).unwrap_or_default() {
                if cookie.name() == "POESESSID" {
                    let _ = win.delete_cookie(cookie);
                }
            }
        }
    }
    use_session(&app, None);
    trace(&app, "signed out of pathofexile.com");
    Ok(())
}

/// Signed in to pathofexile.com right now (checks the saved sign-in if not yet).
#[tauri::command]
pub async fn vs_session(app: AppHandle) -> bool {
    if poe_signed_in(&app) {
        return true;
    }
    refresh_session(&app).await
}

/// Takes the session from the browser's cookies again and checks it works.
pub(crate) async fn refresh_session(app: &AppHandle) -> bool {
    let check = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let Some(win) = check.get_webview_window(MAIN_LABEL) else { return false };
        let Some(session) = session_cookie(&win) else { return false };
        if session_works(&session) {
            use_session(&check, Some(&session));
            trace(&check, "signed in to pathofexile.com (checked again)");
            return true;
        }
        false
    })
    .await
    .unwrap_or(false)
}

/// The POESESSID cookie the webviews hold. Never on the main thread.
fn session_cookie(win: &WebviewWindow) -> Option<String> {
    let url: tauri::Url = POE_SITE.parse().ok()?;
    win.cookies_for_url(url)
        .ok()?
        .into_iter()
        .find(|c| c.name() == "POESESSID" && !c.value().is_empty())
        .map(|c| c.value().to_string())
}

/// True when pathofexile.com treats this session as signed in.
fn session_works(session: &str) -> bool {
    let cookie = format!("POESESSID={session}");
    tauri::async_runtime::block_on(async move {
        let Ok(client) = reqwest::Client::builder().user_agent(USER_AGENT).timeout(Duration::from_secs(15)).build() else {
            return false;
        };
        match client.get(format!("{POE_SITE}my-account")).header("Cookie", cookie).send().await {
            Ok(response) => response.status().is_success() && !response.url().path().starts_with("/login"),
            Err(_) => false,
        }
    })
}

fn use_session(app: &AppHandle, session: Option<&str>) {
    let p = poe(app);
    let Ok(url) = POE_SITE.parse::<reqwest::Url>() else { return };
    match session {
        Some(value) => p.jar.add_cookie_str(&format!("POESESSID={value}; Path=/; Secure; HttpOnly"), &url),
        None => p.jar.add_cookie_str("POESESSID=; Path=/; Max-Age=0", &url),
    }
    p.signed_in.store(session.is_some(), Ordering::SeqCst);
    send(app, "poe-session", json!({ "signedIn": session.is_some() }));
}

fn watch_login(app: &AppHandle) {
    let started = Instant::now();
    let mut tried: Option<String> = None;
    while started.elapsed() < Duration::from_secs(30 * 60) {
        thread::sleep(Duration::from_millis(800));
        let Some(win) = app.get_webview_window(LOGIN_LABEL) else { return };
        if win.url().map(|u| u.path().starts_with("/login")).unwrap_or(true) {
            continue;
        }
        let Some(session) = session_cookie(&win) else { continue };
        if tried.as_deref() == Some(session.as_str()) {
            continue;
        }
        tried = Some(session.clone());
        if session_works(&session) {
            use_session(app, Some(&session));
            trace(app, "signed in to pathofexile.com");
            read_account_name(app, &win);
            let _ = win.destroy();
            // back to Vaal Street, now signed in
            crate::overlay::show(app);
            return;
        }
    }
}

/// Finds the account name ("Name#1234") on the player's account page.
const ACCOUNT_READER: &str = r#"(() => {
  if (location.hostname !== 'www.pathofexile.com') return;
  let name = '';
  const links = [...document.querySelectorAll('a[href*="/account/view-profile/"]')];
  for (const a of links) {
    const t = (a.textContent || '').trim();
    if (/^[^\s#]{2,}#\d{3,5}$/.test(t)) { name = t; break; }
  }
  if (!name) {
    const m = (document.body ? document.body.innerText : '').match(/[^\s#:]{2,30}#\d{4}\b/);
    if (m) name = m[0];
  }
  if (!name && links.length) {
    const h = decodeURIComponent((links[0].getAttribute('href').split('/account/view-profile/')[1] || '').split(/[\/?#]/)[0]);
    if (h) name = h.replace(/-(\d{4})$/, '#$1');
  }
  if (name) location.href = 'https://vaalstreet.invalid/account#' + encodeURIComponent(name);
})()"#;

fn read_account_name(app: &AppHandle, win: &WebviewWindow) {
    crate::wealth::forget_account_in_memory();
    if let Ok(url) = format!("{POE_SITE}my-account").parse::<tauri::Url>() {
        let _ = win.navigate(url);
    }
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(15) {
        thread::sleep(Duration::from_millis(1000));
        if crate::wealth::known_account().is_some() {
            return;
        }
        let _ = win.eval(ACCOUNT_READER);
    }
    trace(app, "the account name was not found on the account page");
}

/// At start: a session kept from last time is used again if it still works.
pub fn restore_session(app: &AppHandle) {
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(2));
        let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
        let Some(session) = session_cookie(&win) else { return };
        if session_works(&session) {
            use_session(&app, Some(&session));
            trace(&app, "signed in to pathofexile.com (from last time)");
        }
    });
}

/// "%7B%22a%22..." back to text.
pub(crate) fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Some(b) = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
