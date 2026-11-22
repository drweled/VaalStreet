// poe.ninja prices: one download of every price list of the league
// ("dense overviews": { currencyOverviews, itemOverviews }, each a list of
// { type, lines: [{ name, variant?, chaos }] }), read again every 30 minutes.
import { shallowRef } from 'vue'
import { fetchText } from './ipc'
import { useLeagues } from './leagues'

/** which kind of thing a poe.ninja list holds (how Vaal Street asks for it) */
function kindOf (type: string): string | null {
  if (type === 'DivinationCard') return 'DIVINATION_CARD'
  if (type === 'SkillGem' || type === 'ImbuedGem') return 'GEM'
  if (type === 'Beast') return 'CAPTURED_BEAST'
  if (type.startsWith('Unique') || type === 'Viscera') return 'UNIQUE'
  // gear bases and normal flasks are priced by item level: never counted
  if (type === 'BaseType' || type === 'Flask' || type === 'ClusterJewel' || type === 'ForbiddenJewel') return null
  return 'ITEM'
}

const key = (ns: string, name: string, variant?: string) => `${ns}\u0000${name}\u0000${variant ?? ''}`

let index = new Map<string, number>()
let loadedFor = ''
let loadedAt = 0
let loading: Promise<void> | null = null

/** chaos per divine (also "prices are ready": it changes whenever they load) */
const xchgRate = shallowRef<number | undefined>(undefined)

async function load (league: string) {
  const res = await fetchText(`https://poe.ninja/poe1/api/economy/current/dense/overviews?league=${encodeURIComponent(league)}&language=en`)
  if (res.status < 200 || res.status >= 300) return
  const data = JSON.parse(res.body) as Record<string, Array<{ type: string, lines?: Array<{ name: string, variant?: string, chaos: number }> }>>
  const next = new Map<string, number>()
  for (const group of Object.values(data)) {
    if (!Array.isArray(group)) continue
    for (const overview of group) {
      const ns = kindOf(overview.type)
      if (!ns) continue
      for (const line of overview.lines ?? []) {
        const k = key(ns, line.name, line.variant)
        if (!next.has(k)) next.set(k, line.chaos)
      }
    }
  }
  if (!next.has(key('ITEM', 'Chaos Orb'))) next.set(key('ITEM', 'Chaos Orb'), 1)
  index = next
  loadedFor = league
  loadedAt = Date.now()
  const divine = next.get(key('ITEM', 'Divine Orb'))
  if (divine && divine >= 30) xchgRate.value = divine
}

/** loads the prices if they are older than 30 minutes or for another league */
function queuePricesFetch () {
  const league = useLeagues().selectedId.value
  if (!league || loading) return
  if (league === loadedFor && Date.now() - loadedAt < 30 * 60_000) return
  loading = load(league).catch(() => {}).finally(() => { loading = null })
}

/** a price in chaos, or null when poe.ninja has no such line */
function findPriceByQuery (q: { ns: string, name: string, variant?: string }): { chaos: number } | null {
  const chaos = index.get(key(q.ns, q.name, q.variant))
  return chaos == null ? null : { chaos }
}

export function usePoeninja () {
  return { xchgRate, findPriceByQuery, queuePricesFetch }
}
