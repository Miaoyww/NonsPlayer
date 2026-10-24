import type { CoverImage, CoverInput, CoverSize } from './types'
export type { CoverInput, CoverSize } from './types'
export function coverOriginal(cover?: CoverInput): string {
  return typeof cover === 'string' ? cover : (cover?.original ?? '')
}
export function validCoverInput(value: unknown): value is CoverImage {
  return (
    !!value &&
    typeof value === 'object' &&
    ['original', 'small', 'normal', 'large', 'exlarge'].every(
      (key) => typeof (value as Record<string, unknown>)[key] === 'string'
    )
  )
}
export function coverCandidates(cover: CoverInput | undefined, size: CoverSize): string[] {
  if (!cover || typeof cover === 'string') return cover ? [cover] : []
  const sizes: CoverSize[] = ['small', 'normal', 'large', 'exlarge']
  const order: CoverSize[] =
    size === 'original'
      ? ['original', 'exlarge', 'large', 'normal', 'small']
      : [...sizes.slice(0, sizes.indexOf(size) + 1).reverse(), 'original']
  return [...new Set(order.map((key) => cover[key]).filter(Boolean))]
}
