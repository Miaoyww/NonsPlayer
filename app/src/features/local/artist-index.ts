import type { LocalEntity } from './local-library'

export const ARTIST_INDEX_LETTERS = [
  'A',
  'B',
  'C',
  'D',
  'E',
  'F',
  'G',
  'H',
  'I',
  'J',
  'K',
  'L',
  'M',
  'N',
  'O',
  'P',
  'Q',
  'R',
  'S',
  'T',
  'U',
  'V',
  'W',
  'X',
  'Y',
  'Z',
  '#'
] as const

export type ArtistIndexLetter = (typeof ARTIST_INDEX_LETTERS)[number]

export interface ArtistIndexGroup {
  letter: ArtistIndexLetter
  items: LocalEntity[]
}

export function artistIndexLetter(name: string): ArtistIndexLetter {
  const initial = name.trim().charAt(0).toUpperCase()
  return /^[A-Z]$/.test(initial) ? (initial as ArtistIndexLetter) : '#'
}

export function groupArtists(items: LocalEntity[]): ArtistIndexGroup[] {
  const grouped = new Map<ArtistIndexLetter, LocalEntity[]>()
  for (const item of items) {
    const letter = artistIndexLetter(item.name)
    const group = grouped.get(letter)
    if (group) group.push(item)
    else grouped.set(letter, [item])
  }
  return ARTIST_INDEX_LETTERS.flatMap((letter) => {
    const group = grouped.get(letter)
    return group ? [{ letter, items: group }] : []
  })
}
