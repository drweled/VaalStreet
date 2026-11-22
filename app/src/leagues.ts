// The leagues (pathofexile.com's public list) and the one picked in the title
// bar, kept in wealth.json.
import { computed, shallowRef } from 'vue'
import { fetchText } from './ipc'
import { wealth, saveWealth } from './wealth'

export interface League { id: string }

const list = shallowRef<League[]>([])

/** the current challenge league (softcore, trade), else Standard */
function defaultLeague (ids: string[]) {
  return ids.find(id => id !== 'Standard' && !/Hardcore|HC |SSF|Solo Self-Found|Ruthless/i.test(id)) ?? 'Standard'
}

const selectedId = computed<string | undefined>({
  get () {
    const ids = list.value.map(l => l.id)
    const saved = wealth.settings.league
    if (saved && (!ids.length || ids.includes(saved))) return saved
    return ids.length ? defaultLeague(ids) : saved
  },
  set (id) {
    wealth.settings.league = id
    saveWealth()
  }
})

async function load () {
  try {
    const res = await fetchText('https://www.pathofexile.com/api/leagues?type=main&realm=pc')
    if (res.status < 200 || res.status >= 300) return
    const data = JSON.parse(res.body) as Array<{ id?: string }>
    list.value = data.filter(l => typeof l.id === 'string').map(l => ({ id: l.id! }))
  } catch {}
}

export function useLeagues () {
  return { list, selectedId, load }
}
