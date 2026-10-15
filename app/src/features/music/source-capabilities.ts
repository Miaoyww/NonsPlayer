import type { SourceDescriptor } from './types'

export function hasPersonalPages(descriptor: SourceDescriptor) {
  return (
    descriptor.capabilities.includes('userLibrary') ||
    descriptor.capabilities.includes('recommendations')
  )
}

export function supportsMusicPage(
  descriptor: SourceDescriptor | undefined,
  page: 'library' | 'discover'
) {
  return !!descriptor?.capabilities.includes(page === 'library' ? 'userLibrary' : 'recommendations')
}
