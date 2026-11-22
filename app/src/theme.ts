// Dark (deep water) or light (shallows). Kept in wealth.json with the rest.
import { shallowRef, watch } from 'vue'
import { wealth, saveWealth } from './wealth'

export const theme = shallowRef<'dark' | 'light'>('dark')

function apply () {
  theme.value = wealth.settings.theme === 'light' ? 'light' : 'dark'
  document.documentElement.dataset.theme = theme.value
}
watch(() => wealth.settings.theme, apply, { immediate: true })

export function toggleTheme () {
  wealth.settings.theme = theme.value === 'dark' ? 'light' : 'dark'
  saveWealth()
}
