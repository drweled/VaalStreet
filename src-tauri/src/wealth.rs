//! Reads the player's own stash tabs from
//! pathofexile.com with the session they signed in with (vs_login), so the
//! trade page can price them with poe.ninja. Also keeps the wealth history and
//! the snapshots in `%APPDATA%\VaalStreet\wealth.json`.
//!
//! pathofexile.com limits these requests per account. The limits are read from
//! every answer (X-Rate-Limit-*) and Vaal Street stays a few requests under each one,
//! never less than 2 s apart, so a sync can never lock the account out of the
//! stash API (or slow down the price checker, which uses other limits).

use std::collections::VecDeque;
use std::fs;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::poe::{data_dir, poe_signed_in, trace};

const STASH_URL: &str = "https://www.pathofexile.com/character-window/get-stash-items";
const MIN_GAP: Duration = Duration::from_millis(2000);

struct Pace {
    /// (hits allowed, period) of every rule pathofexile.com sent
    rules: Vec<(u32, Duration)>,
    sent: VecDeque<Instant>,
    blocked_until: Option<Instant>,
}

static PACE: tokio::sync::Mutex<Pace> = tokio::sync::Mutex::const_new(Pace {
    rules: Vec::new(),
    sent: VecDeque::new(),
    blocked_until: None,
});

/// pathofexile.com/api/stash/â€¦ (Map and Unique tabs and their sub-tabs) has
/// limits of its own ("stash-request-limit": 15 per 10 s, 30 per 5 min when
/// first read), so it is paced on its own, a few under each like the rest.
static PACE_API: tokio::sync::Mutex<Pace> = tokio::sync::Mutex::const_new(Pace {
    rules: Vec::new(),
    sent: VecDeque::new(),
    blocked_until: None,
});
const API_URL: &str = "https://www.pathofexile.com/api/stash";

static ACCOUNT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// How long to wait before the next request may go out.
fn wait_needed(p: &Pace, now: Instant) -> Duration {
    let mut wait = Duration::ZERO;
    if let Some(until) = p.blocked_until {
        wait = wait.max(until.saturating_duration_since(now));
    }
    if let Some(last) = p.sent.back() {
        wait = wait.max((*last + MIN_GAP).saturating_duration_since(now));
    }
    // before the first answer: no more than one request every 5 s
    let rules: Vec<(u32, Duration)> = if p.rules.is_empty() { vec![(2, Duration::from_secs(10))] } else { p.rules.clone() };
    for (max, period) in rules {
        let room = allowed(max) as usize;
        let inside: Vec<&Instant> = p.sent.iter().filter(|t| now.duration_since(**t) < period).collect();
        if inside.len() >= room {
            // wait until the oldest one inside the window leaves it
            let oldest = *inside[inside.len() - room];
            wait = wait.max((oldest + period).saturating_duration_since(now));
        }
    }
    wait
}

/// How many requests of a limit are used: all but a safety margin (27 of 30,
/// 13 of 15), so the account never reaches the limit and is never timed out.
fn allowed(max: u32) -> u32 {
    max.saturating_sub((max / 10).max(2)).max(1)
}

/// "45:60:60,240:240:900" -> [(45, 60 s), (240, 240 s)]
fn parse_rules(text: &str) -> Vec<(u32, Duration)> {
    text.split(',')
        .filter_map(|r| {
            let mut parts = r.trim().split(':');
            let max = parts.next()?.parse::<u32>().ok()?;
            let period = parts.next()?.parse::<u64>().ok()?;
            Some((max, Duration::from_secs(period)))
        })
        .collect()
}

// ------------------------------------------------ the stash, read in the in-app browser
//
// pathofexile.com only treats a stash request as signed in when it carries the
// browser's own cookies (the session cookie alone is answered as "signed out",
// 403). So the requests are made by a hidden Vaal Street browser window that stays
// on pathofexile.com, signed in like the sign-in window (same browser profile).
// Its answer comes back through the address bar (on_navigation), slimmed in the
// page first so it stays small.

pub const STASH_LABEL: &str = "poe-stash";
const STASH_HOME: &str = "https://www.pathofexile.com/my-account";
const RETURN_HOST: &str = "vaalstreet.invalid";

static PAGE_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static WAITING: std::sync::Mutex<Option<(u64, std::sync::mpsc::Sender<Value>)>> = std::sync::Mutex::new(None);

/// Runs in the hidden page: one GET, the limit headers, the answer slimmed.
const FETCH_SCRIPT: &str = r#"(async () => {
  const ID = READ_ID, URL = READ_URL;
  let out;
  try {
    const r = await fetch(URL, { credentials: 'include', headers: { 'X-Requested-With': 'XMLHttpRequest' } });
    const headers = {};
    for (const [k, v] of r.headers) if (/^x-rate-limit|^retry-after/i.test(k)) headers[k.toLowerCase()] = v;
    let body = null;
    if (r.ok) {
      const j = await r.json();
      const keep = (p) => ({ name: p.name, values: p.values });
      const tab = (t) => ({ n: t.n, i: t.i, id: t.id, type: t.type, colour: t.colour, hidden: t.hidden, children: t.children ? t.children.map(tab) : undefined });
      const slim = (it) => ({
        name: it.name, typeLine: it.typeLine, baseType: it.baseType, frameType: it.frameType,
        stackSize: it.stackSize, icon: it.icon, ilvl: it.ilvl, corrupted: it.corrupted, identified: it.identified,
        sockets: it.sockets ? it.sockets.map(s => ({ group: s.group })) : undefined,
        properties: (it.properties || []).filter(p => /^(Level|Quality|Map Tier)$/.test(p.name)).map(keep)
      });
      // /api/stash/â€¦: a special tab (Map, Unique) with its sub-tabs, or one sub-tab
      body = j.stash
        ? { stash: { id: j.stash.id, children: (j.stash.children || []).map(c => ({ id: c.id, metadata: c.metadata })), items: (j.stash.items || []).map(slim) } }
        : { tabs: j.tabs ? j.tabs.map(tab) : undefined, items: (j.items || []).map(slim) };
    }
    out = { id: ID, status: r.status, headers, body, page: location.pathname };
  } catch (e) {
    out = { id: ID, status: 0, error: String(e), headers: {} };
  }
  location.href = 'https://vaalstreet.invalid/stash#' + encodeURIComponent(JSON.stringify(out));
})()"#;

/// The hidden pathofexile.com window, made when first needed.
async fn stash_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    if let Some(win) = app.get_webview_window(STASH_LABEL) {
        return Ok(win);
    }
    PAGE_READY.store(false, std::sync::atomic::Ordering::SeqCst);
    let url = STASH_HOME.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let made = app.clone();
    let built = tauri::async_runtime::spawn_blocking(move || {
        tauri::WebviewWindowBuilder::new(&made, STASH_LABEL, tauri::WebviewUrl::External(url))
            .additional_browser_args(crate::poe::BROWSER_ARGS)
            .title("Vaal Street stash reader")
            .inner_size(400.0, 300.0)
            .visible(false)
            .skip_taskbar(true)
            .focused(false)
            .on_page_load(|win, payload| {
                if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished)
                    && win.url().map(|u| u.host_str() == Some("www.pathofexile.com")).unwrap_or(false)
                {
                    PAGE_READY.store(true, std::sync::atomic::Ordering::SeqCst);
                }
            })
            .on_navigation(move |url| {
                if url.host_str() != Some(RETURN_HOST) {
                    return true;
                }
                let text = url.fragment().map(crate::poe::percent_decode).unwrap_or_default();
                let answer = serde_json::from_str::<Value>(&text).unwrap_or(Value::Null);
                let id = answer.get("id").and_then(Value::as_u64).unwrap_or(0);
                if let Ok(mut w) = WAITING.lock() {
                    if w.as_ref().is_some_and(|(want, _)| *want == id) {
                        if let Some((_, tx)) = w.take() {
                            let _ = tx.send(answer);
                        }
                    }
                }
                false
            })
            .build()
    })
    .await
    .map_err(|e| e.to_string())?;
    let win = built.map_err(|e| {
        trace(app, &format!("Wealthy Weled: could not open the stash reader: {e}"));
        e.to_string()
    })?;
    // wait for pathofexile.com to load in it
    let started = Instant::now();
    while !PAGE_READY.load(std::sync::atomic::Ordering::SeqCst) && started.elapsed() < Duration::from_secs(25) {
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    Ok(win)
}

/// One GET to pathofexile.com through the hidden window: (status, limit headers, slimmed answer).
async fn browser_get(app: &AppHandle, url: &reqwest::Url) -> Result<(u16, serde_json::Map<String, Value>, Value), String> {
    let win = stash_window(app).await?;
    // signed out there: pathofexile.com sent it to its sign-in form
    if win.url().map(|u| u.path().starts_with("/login")).unwrap_or(false) {
        return Err("signed out (sign in again)".into());
    }
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let (tx, rx) = std::sync::mpsc::channel::<Value>();
    if let Ok(mut w) = WAITING.lock() {
        *w = Some((id, tx));
    }
    let script = FETCH_SCRIPT
        .replace("READ_ID", &id.to_string())
        .replace("READ_URL", &Value::from(url.as_str()).to_string());
    win.eval(script).map_err(|e| e.to_string())?;
    let answer = tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(40)))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|_| "pathofexile.com did not answer in time".to_string())?;
    if let Some(error) = answer.get("error").and_then(Value::as_str) {
        return Err(format!("pathofexile.com unreachable: {error}"));
    }
    let status = answer.get("status").and_then(Value::as_u64).unwrap_or(0) as u16;
    let headers = answer.get("headers").and_then(Value::as_object).cloned().unwrap_or_default();
    Ok((status, headers, answer.get("body").cloned().unwrap_or(Value::Null)))
}

/// One paced GET to pathofexile.com's stash API (through the in-app browser).
/// When pathofexile.com asks to wait, or does not answer in time, the same
/// request goes again once the wait is over, so a sync goes on instead of
/// stopping halfway (a Unique tab is many requests).
async fn paced_get(app: &AppHandle, url: reqwest::Url) -> Result<Value, String> {
    match paced_get_once(app, url.clone()).await {
        Err(error) if error.starts_with("pathofexile.com asked to wait") || error.contains("did not answer in time") => {
            trace(app, &format!("Wealthy Weled: trying again after: {error}"));
            paced_get_once(app, url).await
        }
        other => other,
    }
}

async fn paced_get_once(app: &AppHandle, url: reqwest::Url) -> Result<Value, String> {
    let mut pace = if url.path().starts_with("/api/") { PACE_API.lock().await } else { PACE.lock().await };
    loop {
        let wait = wait_needed(&pace, Instant::now());
        if wait.is_zero() {
            break;
        }
        tokio::time::sleep(wait).await;
    }
    let now = Instant::now();
    pace.sent.push_back(now);
    while pace.sent.len() > 400 {
        pace.sent.pop_front();
    }
    let (status, headers, body) = browser_get(app, &url).await?;

    // the limits, as pathofexile.com states them right now
    let header = |name: &str| headers.get(name).and_then(Value::as_str).map(str::to_string);
    if let Some(names) = header("x-rate-limit-rules") {
        let mut rules = Vec::new();
        for name in names.split(',') {
            let name = name.trim().to_lowercase();
            let limits = header(&format!("x-rate-limit-{name}")).map(|t| parse_rules(&t)).unwrap_or_default();
            let state = header(&format!("x-rate-limit-{name}-state")).map(|t| parse_rules(&t)).unwrap_or_default();
            // pathofexile.com counts more than we sent (the website or another
            // tool used the limit too) and the safety margin is reached: wait
            // that whole period out. Our own requests are paced in wait_needed.
            for ((max, period), (used, _)) in limits.iter().zip(state.iter()) {
                let ours = pace.sent.iter().filter(|t| now.duration_since(**t) < *period).count() as u32;
                if *used > ours && *used >= allowed(*max) {
                    let until = now + *period;
                    pace.blocked_until = Some(pace.blocked_until.map_or(until, |b| b.max(until)));
                }
            }
            rules.extend(limits);
        }
        if !rules.is_empty() {
            pace.rules = rules;
        }
    }
    if status == 429 {
        let secs = header("retry-after").and_then(|s| s.parse::<u64>().ok()).unwrap_or(60);
        pace.blocked_until = Some(Instant::now() + Duration::from_secs(secs + 5));
        trace(app, &format!("Wealthy Weled: pathofexile.com asked to wait {secs} s"));
        return Err(format!("pathofexile.com asked to wait {secs} s"));
    }
    drop(pace);
    if status == 401 || status == 403 {
        trace(app, &format!("Wealthy Weled: pathofexile.com answered {status} in the stash reader"));
        // the hidden page may be on an old sign-in: load it again next time
        if let Some(win) = app.get_webview_window(STASH_LABEL) {
            let _ = win.destroy();
        }
        return Err(format!("signed out (sign in again) â€” pathofexile.com answered {status}"));
    }
    if !(200..300).contains(&status) {
        return Err(format!("pathofexile.com answered {status}"));
    }
    Ok(body)
}

fn account_file(app: &AppHandle) -> std::path::PathBuf {
    data_dir(app).join("poe-account.txt")
}

/// The account name ("Name#1234"), read in the sign-in window from the
/// player's account page (trade.rs read_account_name) and kept until they
/// sign out. pathofexile.com has no other way to give it.
fn account(app: &AppHandle) -> Result<String, String> {
    if let Some(name) = known_account() {
        return Ok(name);
    }
    let saved = fs::read_to_string(account_file(app)).unwrap_or_default().trim().to_string();
    if saved.is_empty() {
        return Err("sign in to read your stash".into());
    }
    if let Ok(mut a) = ACCOUNT.lock() {
        *a = Some(saved.clone());
    }
    Ok(saved)
}

pub fn known_account() -> Option<String> {
    ACCOUNT.lock().ok().and_then(|a| a.clone())
}

pub fn forget_account_in_memory() {
    if let Ok(mut a) = ACCOUNT.lock() {
        *a = None;
    }
}

/// From the sign-in window: the account it is signed in to.
pub fn set_account(app: &AppHandle, name: &str) {
    if let Ok(mut a) = ACCOUNT.lock() {
        *a = Some(name.to_string());
    }
    let _ = fs::create_dir_all(data_dir(app));
    let _ = fs::write(account_file(app), name);
    trace(app, "Wealthy Weled: account name read from the account page");
    crate::poe::send(app, "poe-account", json!({ "account": name }));
}

/// Sign out: the session and the account name are forgotten.
#[tauri::command]
pub async fn wealth_sign_out(app: AppHandle) -> Result<(), String> {
    forget_account_in_memory();
    let _ = fs::remove_file(account_file(&app));
    if let Some(win) = app.get_webview_window(STASH_LABEL) {
        let _ = win.destroy();
    }
    crate::poe::vs_logout(app).await
}

fn stash_url(account: &str, league: &str, tabs: bool, index: u32) -> Result<reqwest::Url, String> {
    reqwest::Url::parse_with_params(
        STASH_URL,
        &[
            ("accountName", account),
            ("realm", "pc"),
            ("league", league),
            ("tabs", if tabs { "1" } else { "0" }),
            ("tabIndex", &index.to_string()),
        ],
    )
    .map_err(|e| e.to_string())
}

fn hex(colour: Option<&Value>) -> String {
    let c = |k: &str| colour.and_then(|c| c.get(k)).and_then(Value::as_u64).unwrap_or(120).min(255);
    format!("#{:02x}{:02x}{:02x}", c("r"), c("g"), c("b"))
}

fn tab_list(tabs: &[Value], out: &mut Vec<Value>) {
    for t in tabs {
        if t.get("hidden").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        if let Some(children) = t.get("children").and_then(Value::as_array) {
            tab_list(children, out);
            continue;
        }
        let Some(index) = t.get("i").and_then(Value::as_u64) else { continue };
        out.push(json!({
            "id": t.get("id").and_then(Value::as_str).unwrap_or_default(),
            "index": index,
            "name": t.get("n").and_then(Value::as_str).unwrap_or_default(),
            "type": t.get("type").and_then(Value::as_str).unwrap_or_default(),
            "colour": hex(t.get("colour")),
        }));
    }
}

/// "20 (Max)" / "+20%" -> 20
fn number(text: &str) -> Option<f64> {
    let digits: String = text.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    digits.parse().ok()
}

fn property(item: &Value, name: &str) -> Option<f64> {
    item.get("properties")?
        .as_array()?
        .iter()
        .find(|p| p.get("name").and_then(Value::as_str) == Some(name))?
        .get("values")?
        .get(0)?
        .get(0)?
        .as_str()
        .and_then(number)
}

/// Only what the pricing needs, so a full tab stays small.
fn slim(item: &Value) -> Value {
    let links = item
        .get("sockets")
        .and_then(Value::as_array)
        .map(|s| {
            let mut groups = std::collections::HashMap::new();
            for socket in s {
                *groups.entry(socket.get("group").and_then(Value::as_u64).unwrap_or(0)).or_insert(0u32) += 1;
            }
            groups.values().copied().max().unwrap_or(0)
        })
        .unwrap_or(0);
    json!({
        "name": item.get("name").and_then(Value::as_str).unwrap_or_default(),
        "typeLine": item.get("typeLine").and_then(Value::as_str).unwrap_or_default(),
        "baseType": item.get("baseType").and_then(Value::as_str).unwrap_or_default(),
        "frameType": item.get("frameType").and_then(Value::as_u64).unwrap_or(0),
        "stack": item.get("stackSize").and_then(Value::as_u64).unwrap_or(1),
        "icon": item.get("icon").and_then(Value::as_str).unwrap_or_default(),
        "ilvl": item.get("ilvl").and_then(Value::as_u64).unwrap_or(0),
        "corrupted": item.get("corrupted").and_then(Value::as_bool).unwrap_or(false),
        "identified": item.get("identified").and_then(Value::as_bool).unwrap_or(true),
        "level": property(item, "Level"),
        "quality": property(item, "Quality"),
        "mapTier": property(item, "Map Tier"),
        "links": links,
    })
}

/// Signed in, and the account name.
#[tauri::command]
pub async fn wealth_account(app: AppHandle) -> Value {
    if !poe_signed_in(&app) {
        if let Ok(mut a) = ACCOUNT.lock() {
            *a = None;
        }
        return json!({ "signedIn": false });
    }
    match account(&app) {
        Ok(name) => json!({ "signedIn": true, "account": name }),
        Err(error) => json!({ "signedIn": true, "error": error }),
    }
}

/// The league's stash tabs: [{ id, index, name, type, colour }].
#[tauri::command]
pub async fn wealth_tabs(app: AppHandle, league: String) -> Result<Value, String> {
    let account = account(&app)?;
    let answer = paced_get(&app, stash_url(&account, &league, true, 0)?).await?;
    let mut out = Vec::new();
    tab_list(answer.get("tabs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default(), &mut out);
    trace(&app, &format!("Wealthy Weled: {} tabs in {league}", out.len()));
    Ok(Value::Array(out))
}

/// One tab's items (slimmed). Map and Unique tabs (`kind`) are read through
/// /api/stash/â€¦ with the tab's `id`: the old request answers them empty.
#[tauri::command]
pub async fn wealth_tab(app: AppHandle, league: String, index: u32, id: Option<String>, kind: Option<String>) -> Result<Value, String> {
    match (kind.as_deref(), id.as_deref()) {
        (Some("MapStash"), Some(id)) => return map_tab(&app, &league, id).await,
        (Some("UniqueStash"), Some(id)) => return unique_tab(&app, &league, id).await,
        _ => {}
    }
    let account = account(&app)?;
    let answer = paced_get(&app, stash_url(&account, &league, false, index)?).await?;
    let items: Vec<Value> = answer.get("items").and_then(Value::as_array).map(|a| a.iter().map(slim).collect()).unwrap_or_default();
    Ok(Value::Array(items))
}

/// https://www.pathofexile.com/api/stash/<league>/<id>[/<child>]
fn api_url(league: &str, ids: &[&str]) -> Result<reqwest::Url, String> {
    let mut url = reqwest::Url::parse(API_URL).map_err(|e| e.to_string())?;
    {
        let mut path = url.path_segments_mut().map_err(|_| "bad url".to_string())?;
        path.push(league);
        for id in ids {
            path.push(id);
        }
    }
    Ok(url)
}

/// The Map tab: its sub-tab list already says which map and how many
/// ({"items":60,"map":{"section":"tier11","name":"Map (Tier 11)"}}): one request.
async fn map_tab(app: &AppHandle, league: &str, id: &str) -> Result<Value, String> {
    let answer = paced_get(app, api_url(league, &[id])?).await?;
    let children = answer.pointer("/stash/children").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut items = Vec::new();
    for child in children {
        let Some(meta) = child.get("metadata") else { continue };
        let count = meta.get("items").and_then(Value::as_u64).unwrap_or(0);
        let Some(name) = meta.pointer("/map/name").and_then(Value::as_str) else { continue };
        if count == 0 {
            continue;
        }
        let section = meta.pointer("/map/section").and_then(Value::as_str).unwrap_or("");
        let unique = section == "unique";
        // poe.ninja lists every map as tier "0" ("Map (Tier 11)", T0)
        let tier = name.split("(Tier ").nth(1).and_then(|t| t.split(')').next()).unwrap_or("0");
        let raw = json!({
            "name": if unique { name } else { "" },
            "typeLine": name,
            "baseType": name,
            "frameType": if unique { 3 } else { 0 },
            "stackSize": count,
            "properties": [{ "name": "Map Tier", "values": [[tier, 0]] }],
        });
        items.push(slim(&raw));
    }
    trace(app, &format!("Wealthy Weled: Map tab read ({} kinds of map)", items.len()));
    Ok(Value::Array(items))
}

/// The Unique tab: its sub-tabs, then each sub-tab that holds items.
async fn unique_tab(app: &AppHandle, league: &str, id: &str) -> Result<Value, String> {
    let answer = paced_get(app, api_url(league, &[id])?).await?;
    let parent = answer.pointer("/stash/id").and_then(Value::as_str).unwrap_or(id).to_string();
    let children = answer.pointer("/stash/children").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut items: Vec<Value> = answer.pointer("/stash/items").and_then(Value::as_array).map(|a| a.iter().map(slim).collect()).unwrap_or_default();
    let mut read = 0;
    for child in children {
        let has = child.pointer("/metadata/items").and_then(Value::as_u64).unwrap_or(1);
        let Some(child_id) = child.get("id").and_then(Value::as_str) else { continue };
        if has == 0 {
            continue;
        }
        let page = paced_get(app, api_url(league, &[&parent, child_id])?).await?;
        if let Some(list) = page.pointer("/stash/items").and_then(Value::as_array) {
            items.extend(list.iter().map(slim));
        }
        read += 1;
    }
    trace(app, &format!("Wealthy Weled: Unique tab read ({read} sub-tabs, {} items)", items.len()));
    Ok(Value::Array(items))
}

/// Seconds until the next stash request may go out (for the progress text).
#[tauri::command]
pub async fn wealth_wait() -> f64 {
    let now = Instant::now();
    let a = wait_needed(&*PACE.lock().await, now);
    let b = wait_needed(&*PACE_API.lock().await, now);
    // a sync reads either kind next: the longer wait is the honest countdown
    a.max(b).as_secs_f64()
}

fn wealth_file(app: &AppHandle) -> std::path::PathBuf {
    data_dir(app).join("wealth.json")
}

/// The saved history, tab choice and snapshots (JSON), if any.
#[tauri::command]
pub fn wealth_load(app: AppHandle) -> Option<String> {
    fs::read_to_string(wealth_file(&app)).ok()
}

#[tauri::command]
pub fn wealth_save(app: AppHandle, json: String) -> Result<(), String> {
    serde_json::from_str::<Value>(&json).map_err(|e| e.to_string())?;
    let _ = fs::create_dir_all(data_dir(&app));
    let path = wealth_file(&app);
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_and_pace() {
        let rules = parse_rules("45:60:60,240:240:900");
        assert_eq!(rules, vec![(45, Duration::from_secs(60)), (240, Duration::from_secs(240))]);
        let now = Instant::now();
        let mut p = Pace { rules: vec![(4, Duration::from_secs(10))], sent: VecDeque::new(), blocked_until: None };
        assert!(wait_needed(&p, now).is_zero());
        p.sent.push_back(now);
        assert!(wait_needed(&p, now) >= Duration::from_millis(1900));
        p.sent.push_back(now + Duration::from_secs(3));
        // 2 of 4 (all but the margin) used inside 10 s: wait for the first to leave the window
        let w = wait_needed(&p, now + Duration::from_secs(5));
        assert_eq!(w, Duration::from_secs(5));
    }

    #[test]
    fn numbers() {
        assert_eq!(number("20 (Max)"), Some(20.0));
        assert_eq!(number("+23%"), Some(23.0));
    }
}
