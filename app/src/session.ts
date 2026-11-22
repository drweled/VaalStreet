// Signed in to pathofexile.com or not. The sign-in itself happens on
// pathofexile.com's own page, in a window the program opens.
import { shallowRef } from 'vue'
import { invoke, onEvent } from './ipc'

export const signedIn = shallowRef(false)

onEvent('poe-session', (e) => { signedIn.value = e.signedIn })
void invoke<boolean>('vs_session').then(v => { signedIn.value = v }).catch(() => {})

/** opens pathofexile.com's sign-in page */
export function signIn () {
  void invoke('vs_login').catch(() => {})
}

/** asks again (a saved sign-in is checked a moment after start) */
export async function recheckSession (): Promise<boolean> {
  const v = await invoke<boolean>('vs_session').catch(() => null)
  if (v != null) signedIn.value = v
  return signedIn.value
}
