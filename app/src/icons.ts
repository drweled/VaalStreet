// Pictures for the few things Vaal Street shows before a tab was read (a tab's
// kind) and the chaos / divine signs. Path of Exile's own images (GGG's image
// server, web.poecdn.com), as poe.ninja lists them. Items read from the stash
// bring their own picture.

const CDN = 'https://web.poecdn.com'

export const KNOWN_ICONS: Record<string, string> = {
  'Chaos Orb': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvQ3VycmVuY3lSZXJvbGxSYXJlIiwidyI6MSwiaCI6MSwic2NhbGUiOjF9XQ/d119a0d734/CurrencyRerollRare.png`,
  'Divine Orb': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvQ3VycmVuY3lNb2RWYWx1ZXMiLCJ3IjoxLCJoIjoxLCJzY2FsZSI6MX1d/e1a54ff97d/CurrencyModValues.png`,
  'Tainted Chaos Orb': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvSGVsbHNjYXBlL0hlbGxzY2FwZUNoYW9zT3JiIiwidyI6MSwiaCI6MSwic2NhbGUiOjF9XQ/bd934ad92b/HellscapeChaosOrb.png`,
  'Mirror of Kalandra': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvQ3VycmVuY3lEdXBsaWNhdGUiLCJ3IjoxLCJoIjoxLCJzY2FsZSI6MX1d/7111e35254/CurrencyDuplicate.png`,
  'Prismatic Catalyst': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvQ2F0YWx5c3RzL1ByaXNtYXRpY0NhdGFseXN0IiwidyI6MSwiaCI6MSwic2NhbGUiOjF9XQ/5e51d41a0e/PrismaticCatalyst.png`,
  "Rogue's Marker": `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvSGVpc3QvSGVpc3RDb2luQ3VycmVuY3kiLCJ3IjoxLCJoIjoxLCJzY2FsZSI6MX1d/987a8953f9/HeistCoinCurrency.png`,
  'Headhunter': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQmVsdHMvSGVhZGh1bnRlciIsInciOjIsImgiOjEsInNjYWxlIjoxfV0/e9542ec6ee/Headhunter.png`,
  'Simulacrum Splinter': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvTWFwcy9EZWxpcml1bVNwbGludGVyIiwidyI6MSwiaCI6MSwic2NhbGUiOjF9XQ/ae73b9445e/DeliriumSplinter.png`,
  'Fragment of the Phoenix': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvTWFwcy9BdGxhc01hcHMvRnJhZ21lbnRQaG9lbml4IiwidyI6MSwiaCI6MSwic2NhbGUiOjF9XQ/bf64bb03f8/FragmentPhoenix.png`,
  'Empower Support': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvR2Vtcy9TdXBwb3J0L0VtcG93ZXIiLCJ3IjoxLCJoIjoxLCJzY2FsZSI6MX1d/c2b1b38b15/Empower.png`,
  'Blighted Map': `${CDN}/gen/image/WzI4LDE0LHsiZiI6IjJESXRlbXMvTWFwcy9BdGxhczJNYXBzL05ldy9NYXBOdW1iZXJzMTYiLCJ3IjoxLCJoIjoxLCJzY2FsZSI6MSwibW4iOjI0LCJtdCI6MCwibWIiOnRydWV9XQ/d53d25cf47/MapNumbers16.png`,
  'Lethal Pride': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvSmV3ZWxzL0thcnVpQ2l2aWxpemF0aW9uIiwidyI6MSwiaCI6MSwic2NhbGUiOjF9XQ/04aefd20e6/KaruiCivilization.png`,
  'Divine Life Flask': `${CDN}/gen/image/WzksMTQseyJmIjoiMkRJdGVtcy9GbGFza3MvbGlmZWZsYXNrMTEiLCJ3IjoxLCJoIjoyLCJzY2FsZSI6MSwibGV2ZWwiOjF9XQ/54ba3c70ae/lifeflask11.png`,
  'Dense Fossil': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvRGVsdmUvRGVuc2VGb3NzaWwiLCJzY2FsZSI6MX1d/ac39aaf3cd/DenseFossil.png`,
  'Golden Oil': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvT2lscy9Hb2xkZW5PaWwiLCJzY2FsZSI6MX1d/263a5d73da/GoldenOil.png`,
  'Deafening Essence of Greed': `${CDN}/gen/image/WzI1LDE0LHsiZiI6IjJESXRlbXMvQ3VycmVuY3kvRXNzZW5jZS9HcmVlZDciLCJzY2FsZSI6MX1d/9d7e9a3b11/Greed7.png`,
  'Divination Card': `${CDN}/image/Art/2DItems/Divination/InventoryIcon.png`
}

export function knownIcon (name: string): string {
  return KNOWN_ICONS[name] ?? ''
}

export const CHAOS_ICON = KNOWN_ICONS['Chaos Orb']
export const DIVINE_ICON = KNOWN_ICONS['Divine Orb']
