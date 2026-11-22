// The link between the Vaal Street window and the program (Tauri): calling the
// program's commands, and its events ("vs-event", { name, payload }).

interface TauriGlobal {
  core: { invoke: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T> }
  event: { listen: <T>(name: string, cb: (e: { payload: T }) => void) => Promise<() => void> }
}

const tauri = () => (window as unknown as { __TAURI__?: TauriGlobal }).__TAURI__

export function invoke<T> (cmd: string, args?: Record<string, unknown>): Promise<T> {
  const t = tauri()
  return t ? t.core.invoke<T>(cmd, args) : Promise.reject(new Error('Vaal Street is not running'))
}

/** what the program tells the window */
export interface VsEvents {
  'poe-session': { signedIn: boolean }
  'poe-account': { account: string }
  'update-progress': { downloaded: number, total: number | null, installing?: boolean }
}

type Handler = (payload: never) => void
const handlers = new Map<string, Set<Handler>>()
let listening = false

export function onEvent<K extends keyof VsEvents> (name: K, cb: (payload: VsEvents[K]) => void) {
  let set = handlers.get(name)
  if (!set) handlers.set(name, set = new Set())
  set.add(cb as Handler)
  if (!listening && tauri()) {
    listening = true
    void tauri()!.event.listen<{ name: string, payload: never }>('vs-event', (e) => {
      handlers.get(e.payload.name)?.forEach(h => h(e.payload.payload))
    })
  }
}

/** a web request made by the program (pages may not call these sites themselves) */
export interface Fetched { status: number, statusText: string, headers: Array<[string, string]>, body: string }

export function fetchText (url: string): Promise<Fetched> {
  return invoke<Fetched>('vs_fetch', { url })
}

/** the same, kept on disk and read again at most every `maxAgeSecs` */
export function fetchCached (url: string, maxAgeSecs: number): Promise<Fetched> {
  return invoke<Fetched>('vs_cached', { url, maxAgeSecs })
}
