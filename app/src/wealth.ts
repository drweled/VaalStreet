// Vaal Street: the player's stash, priced with poe.ninja.
// The tabs are read by wealth.rs (pathofexile.com, the player's own session,
// paced to half of every limit). Each tab is kept as a list of priceable
// things and their counts, and priced again from poe.ninja whenever it is
// shown, so the value follows the market without reading the stash again.
// History, the tab choice, the runs and the settings live in
// %APPDATA%\VaalStreet\wealth.json.
import { reactive, shallowRef } from 'vue'
import { invoke, fetchCached } from './ipc'

export { invoke }

/** support gems poe.ninja lists by level only (no quality) */
const SPECIAL_SUPPORT_GEM = ['Empower Support', 'Enlighten Support', 'Enhance Support']

export interface StashTab { id: string, index: number, name: string, type: string, colour: string }

/** an item as wealth.rs sends it */
export interface SlimItem {
  name: string
  typeLine: string
  baseType: string
  frameType: number
  stack: number
  icon: string
  ilvl: number
  corrupted: boolean
  identified: boolean
  level: number | null
  quality: number | null
  mapTier: number | null
  links: number
}

/** something poe.ninja has a price for, and how to ask for it */
export interface Priceable {
  key: string
  name: string
  icon: string
  /** poe.ninja lookups, first that answers wins */
  ask: Array<{ ns: string, name: string, variant?: string }>
}

/** one tab, as kept: what it holds that can be priced */
export interface TabHoldings { at: number, entries: Array<Priceable & { qty: number }> }

/** one sync: the total, and each tab's share (chaos) so the graph can count only ticked tabs */
export interface WealthPoint { t: number, chaos: number, div: number, tabs?: Record<string, number> }

export interface Snapshot {
  id: string
  league: string
  started: number
  /** null while it runs */
  ended: number | null
  startChaos: number
  startDiv: number
  endChaos: number
  endDiv: number
  /** key -> count at the start / at the last sync */
  startQty: Record<string, number>
  endQty: Record<string, number>
  /** key -> name and icon, for the change list */
  names: Record<string, { name: string, icon: string, ask: Priceable['ask'] }>
}

/** one snapshot of a run: what the run's tabs held, and the prices then */
export interface RunSnap {
  t: number
  /** chaos per divine then */
  rate: number
  /** key -> count, in every tab of the run (paying tab + loot tabs) */
  qty: Record<string, number>
  /** key -> count, in the paying tab only */
  payQty: Record<string, number>
  /** key -> count, in the loot (ticked) tabs only — never the paying tab */
  lootQty?: Record<string, number>
  /** key -> chaos each, then */
  price: Record<string, number>
}

/**
 * A run (a strategy): 1) snapshot at the start, 2) the buying phase (maps,
 * scarabs… bought with the paying tab) ends with a snapshot, 3) the run
 * itself ends with a snapshot. Investment = what left the paying tab while
 * buying; revenue = what came into the other ticked tabs from start to
 * finish, plus — only when the paying tab is ticked too — what came into the
 * paying tab from "Done buying" to finish (so the spending is not counted
 * twice); profit = revenue − investment.
 */
export interface Run {
  id: string
  league: string
  payTabId: string
  payTabName: string
  /** the ticked tabs besides the paying tab */
  lootTabIds: string[]
  /** the paying tab was ticked too: its loot during the run (Done buying →
   *  Finish) counts as revenue as well */
  payCounts?: boolean
  phase: 'buying' | 'running' | 'done'
  a: RunSnap
  b?: RunSnap
  c?: RunSnap
  names: Record<string, { name: string, icon: string, ask: Priceable['ask'] }>
}

export interface WealthState {
  /** league -> chosen tab ids */
  selected: Record<string, string[]>
  /** league -> tabs (last listing) */
  tabs: Record<string, StashTab[]>
  /** league -> tab id -> holdings */
  holdings: Record<string, Record<string, TabHoldings>>
  /** league -> totals after each sync */
  history: Record<string, WealthPoint[]>
  /** the first version's sessions (kept in the file, no longer shown) */
  snapshots: Snapshot[]
  /** runs (Snapshots tab) */
  runs: Run[]
  /** league -> the tab investments are paid from (remembered) */
  payTab: Record<string, string>
  /** league -> when the last sync finished */
  lastSync: Record<string, number>
  /** the league picked in the title bar, light or dark */
  settings: { league?: string, theme?: 'dark' | 'light' }
  autoSyncMin: number
}

export const wealth = reactive<WealthState>({
  selected: {},
  tabs: {},
  holdings: {},
  history: {},
  snapshots: [],
  runs: [],
  payTab: {},
  lastSync: {},
  settings: {},
  autoSyncMin: 10
})
export const wealthLoaded = shallowRef(false)

/**
 * Reads trade\wealth.json once (every caller waits for the same read).
 * Nothing is saved before it finished: a save before the load would write an
 * empty state over the player's ticked tabs, holdings and history.
 */
let loading: Promise<void> | null = null
export function loadWealth (): Promise<void> {
  loading ??= (async () => {
    let failed = false
    try {
      const json = await invoke<string | null>('wealth_load')
      if (json) Object.assign(wealth, JSON.parse(json))
    } catch {
      failed = true
    }
    wealth.selected ??= {}
    wealth.tabs ??= {}
    wealth.holdings ??= {}
    wealth.history ??= {}
    wealth.snapshots ??= []
    wealth.runs ??= []
    wealth.payTab ??= {}
    wealth.lastSync ??= {}
    wealth.settings ??= {}
    // a file that could not be read is never overwritten (Vaal Street not
    // running, or a broken file): saving stays off
    wealthLoaded.value = !failed
    if (failed) loading = null
  })()
  return loading
}

let saveTimer: ReturnType<typeof setTimeout> | undefined
export function saveWealth () {
  clearTimeout(saveTimer)
  saveTimer = setTimeout(() => {
    void loadWealth().then(() => {
      if (!wealthLoaded.value) return
      void invoke('wealth_save', { json: JSON.stringify(wealth) }).catch(() => {})
    })
  }, 400)
}

// ------------------------------------------------ map pictures (poe.ninja)
// The Map stash tab sends counts per map, no pictures; poe.ninja's map lists
// have one for every map ("Map (Tier 11)", "Nightmare Map", blighted maps…).
// Read once a day per league, kept on disk by the program.
export const mapIcons = shallowRef<Record<string, string>>({})
let mapIconsLeague = ''
export async function loadMapIcons (league: string) {
  if (!league || mapIconsLeague === league) return
  mapIconsLeague = league
  const out: Record<string, string> = {}
  for (const type of ['Map', 'BlightedMap', 'BlightRavagedMap', 'UniqueMap']) {
    try {
      const url = `https://poe.ninja/poe1/api/economy/stash/current/item/overview?league=${encodeURIComponent(league)}&type=${type}`
      const res = await fetchCached(url, 24 * 3600)
      if (res.status < 200 || res.status >= 300) continue
      for (const line of (JSON.parse(res.body) as { lines?: Array<{ name: string, icon?: string }> }).lines ?? []) {
        if (line.icon && !out[line.name]) out[line.name] = line.icon
      }
    } catch {}
  }
  mapIcons.value = out
}

// ------------------------------------------------ which poe.ninja line an item is

/**
 * poe.ninja's gem lines: "20/20", "21/20c", "1/23c", "20", "1", "4c" (Empower…).
 * The closest first, then plainer ones, so every gem finds a price.
 */
function gemVariants (it: SlimItem): string[] {
  const name = it.typeLine
  const level = it.level ?? 1
  const special = SPECIAL_SUPPORT_GEM.includes(name)
  const lv = (special || level >= 20) ? `${level}` : '1'
  const q = it.quality ?? 0
  const qs = q >= 20 ? `${q}` : q >= 16 ? '20' : q ? `${q}` : ''
  const c = it.corrupted ? 'c' : ''
  const out = [
    qs && !special ? `${lv}/${qs}${c}` : '',
    `${lv}${c}`,
    qs && !special ? `${lv}/${qs}` : '',
    `${lv}`,
    level >= 20 ? `20${c}` : '',
    '1'
  ]
  return [...new Set(out.filter(Boolean))]
}

/** "Lavish Wombgift" is priced per item level: 40, 68, 75, 84 */
function wombgiftLevel (ilvl: number) {
  return [84, 75, 68, 40].find(b => ilvl >= b) ?? 40
}

export function priceableOf (it: SlimItem): Priceable | null {
  const icon = it.icon
  // 3 unique, 9 relic
  if (it.frameType === 3 || it.frameType === 9) {
    if (!it.name) return null // unidentified: which unique is unknown
    const base = it.baseType || it.typeLine
    const ask: Priceable['ask'] = []
    // unique maps are listed as "T0"
    if (it.mapTier != null) ask.push({ ns: 'UNIQUE', name: it.name, variant: 'T0' })
    if (it.links >= 5) ask.push({ ns: 'UNIQUE', name: it.name, variant: `${base}, ${it.links}L` })
    ask.push({ ns: 'UNIQUE', name: it.name, variant: base }, { ns: 'UNIQUE', name: it.name })
    const links = it.links >= 5 ? ` (${it.links}L)` : ''
    return { key: `U|${it.name}|${base}|${it.links >= 5 ? it.links : 0}`, name: `${it.name}${links}`, icon, ask }
  }
  if (it.frameType === 4) {
    const variants = gemVariants(it)
    const name = it.typeLine
    return {
      key: `G|${name}|${variants[0]}`,
      name: `${name} - ${variants[0]}`,
      icon,
      // skill gems, then transfigured / imbued lists (same names)
      ask: [...variants.map(variant => ({ ns: 'GEM', name, variant })), { ns: 'GEM', name }]
    }
  }
  if (it.frameType === 6) {
    return { key: `D|${it.typeLine}`, name: it.typeLine, icon, ask: [{ ns: 'DIVINATION_CARD', name: it.typeLine }] }
  }
  const base = it.baseType || it.typeLine
  // flasks: only uniques count (above); normal and magic flasks never
  if (/\bFlask\b/.test(base)) return null
  // a map: it has a tier, or its name says so (the Blight tab sends
  // "Blighted Map (Tier 15)" with no "Map Tier" line)
  const tierInName = /\bMap \(Tier (\d+)\)/.exec(it.typeLine)
  if (it.mapTier == null && tierInName && it.frameType <= 2) it = { ...it, mapTier: Number(tierInName[1]) }
  if (it.mapTier != null && it.frameType <= 2) {
    // poe.ninja: "Map (Tier 11)", "Blighted Map (Tier 5)", "Nightmare Map",
    // "Drox Map (Tier 16)" … all listed as "T0, Gen-24" (Vaal temples "T0, Atlas")
    const special = /(Blight-ravaged|Blighted) Map \(Tier \d+\)/.exec(it.typeLine)?.[0]
    const names = [...new Set([special, base, /\(Tier/.test(base) ? '' : `Map (Tier ${it.mapTier})`].filter((n): n is string => Boolean(n)))]
    const ask: Priceable['ask'] = []
    for (const name of names) ask.push({ ns: 'ITEM', name, variant: 'T0, Gen-24' }, { ns: 'ITEM', name, variant: 'T0, Atlas' }, { ns: 'ITEM', name })
    return { key: `M|${names[0]}|${it.mapTier}`, name: names[0], icon, ask }
  }
  if (/Wombgift$/.test(base)) {
    const lv = wombgiftLevel(it.ilvl)
    return {
      key: `W|${base}|${lv}`,
      name: `${base} (level ${lv})`,
      icon,
      ask: [{ ns: 'ITEM', name: base, variant: String(lv) }, { ns: 'ITEM', name: base }]
    }
  }
  // currency, fragments, threads, scarabs, essences, oils, fossils, delirium
  // orbs, … by their exact name, whatever kind of item the stash says it is.
  // Magic and rare gear never matches a poe.ninja line, so it stays unpriced
  // (like Wealthy Exile).
  {
    const ask: Priceable['ask'] = [{ ns: 'ITEM', name: base }]
    if (it.typeLine && it.typeLine !== base) ask.push({ ns: 'ITEM', name: it.typeLine })
    // beasts (Bestiary) are their own list
    ask.push({ ns: 'CAPTURED_BEAST', name: it.typeLine || base })
    return { key: `I|${base}`, name: base, icon, ask }
  }
}

/** a tab's items, folded into priceable lines with counts */
export function holdingsOf (items: SlimItem[]): TabHoldings {
  const by = new Map<string, Priceable & { qty: number }>()
  for (const it of items) {
    const p = priceableOf(it)
    if (!p) continue
    const had = by.get(p.key)
    if (had) had.qty += it.stack || 1
    else by.set(p.key, { ...p, qty: it.stack || 1 })
  }
  return { at: Date.now(), entries: [...by.values()] }
}

/** sample data for the preview (no game, no account) */
export function sampleHoldings (): Record<string, TabHoldings> {
  const e = (name: string, qty: number) => ({ key: `I|${name}`, name, icon: '', qty, ask: [{ ns: 'ITEM', name }] })
  return {
    t1: { at: Date.now(), entries: [e('Divine Orb', 12), e('Orb of Alteration', 536), e('Chromatic Orb', 39), e('Orb of Scouring', 68), e('Chaos Orb', 46)] },
    t2: { at: Date.now(), entries: [e('Blight Thread', 34), e('Glassblower\'s Bauble', 17)] }
  }
}
