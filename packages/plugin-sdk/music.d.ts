export type CoverSize = 'small' | 'normal' | 'large' | 'exlarge' | 'original'
export interface CoverImage {
  original: string
  small: string
  normal: string
  large: string
  exlarge: string
}
/** Strings are compatibility artwork from saved state or local paths. */
export type CoverInput = CoverImage | string
/** ADR 0006 v1 source-neutral music DTOs; IDs and cursors are opaque strings. */
export type EntityKind = 'track' | 'playlist' | 'album' | 'artist'
export interface EntityRef {
  source: string
  kind: EntityKind
  id: string
}
export interface AccountRef {
  source: string
  id: string
}
export interface AccountRecord {
  reference: AccountRef
  displayName: string
  avatar: CoverInput
}
export interface MusicCredit {
  name: string
  reference: EntityRef | null
}
export interface MusicTrack {
  reference: EntityRef
  title: string
  aliases: string[]
  artist: string
  artists: MusicCredit[]
  album: string
  albumReference: EntityRef | null
  durationMs: number
  cover: CoverInput
  associations: EntityRef[]
}
export interface MusicEntity {
  reference: EntityRef
  name: string
  cover: CoverInput
  subtitle: string
  trackCount: number
  creator: AccountRef | null
  liked: boolean
  playCount: number | null
  publishedAt: number | null
  artists: MusicCredit[]
}
export interface PageRequest {
  cursor: string | null
  limit: number
}
export interface Page<T> {
  items: T[]
  nextCursor: string | null
}
export type MusicErrorCode =
  | 'unauthenticated'
  | 'notFound'
  | 'permissionDenied'
  | 'regionRestricted'
  | 'rateLimited'
  | 'network'
  | 'unsupported'
  | 'sourceUnavailable'
  | 'cancelled'
  | 'deadlineExceeded'
  | 'staleContext'
  | 'invalidData'
  | 'internal'
export interface MusicError {
  code: MusicErrorCode
  retryAfterMs: number | null
}
export type MusicReadRequest =
  | { operation: 'search'; keyword: string; kind: EntityKind; page: PageRequest }
  | { operation: 'suggestions'; keyword: string }
  | { operation: 'detail'; entity: EntityRef }
  | { operation: 'tracks'; entity: EntityRef; page: PageRequest }
  | { operation: 'artistAlbums' | 'artistTracks'; artist: EntityRef; page: PageRequest }
  | { operation: 'trackInformation'; track: EntityRef }
  | { operation: 'librarySummary' | 'favorites' | 'categories' | 'radar' }
  | {
      operation: 'libraryCollections'
      kind: Exclude<EntityKind, 'track'>
      filter: string
      page: PageRequest
    }
  | { operation: 'history'; week: boolean; page: PageRequest }
  | {
      operation: 'recommendedPlaylists'
      section: string
      category: string
      order: string
      page: PageRequest
    }
  | { operation: 'recommendedTracks'; kind: string }
export type MusicWriteRequest =
  | { operation: 'setFavorite'; track: EntityRef; liked: boolean }
  | { operation: 'createPlaylist'; name: string; private: boolean }
  | { operation: 'updatePlaylist'; playlist: EntityRef; name: string; description: string }
  | { operation: 'deletePlaylist'; playlist: EntityRef }
  | { operation: 'playlistTrack'; playlist: EntityRef; track: EntityRef; add: boolean }
  | { operation: 'dislike'; track: EntityRef }
export type MusicRequest = MusicReadRequest | MusicWriteRequest
export interface WriteImpact {
  operations: string[]
  entities: EntityRef[]
}
export type MusicResponse =
  | {
      type: 'tracks'
      data: Page<MusicTrack> & { total: number | null; description: string | null }
    }
  | { type: 'entities'; data: Page<MusicEntity> }
  | { type: 'entity'; data: MusicEntity }
  | {
      type: 'detail'
      data: { item: MusicEntity; description: string | null; albumCount: number | null }
    }
  | {
      type: 'summary'
      data: {
        account: AccountRecord
        likedPlaylist: MusicEntity | null
        likedTracks: MusicTrack[]
        likedError: MusicError | null
      }
    }
  | { type: 'favorites'; data: EntityRef[] }
  | { type: 'suggestions'; data: string[] }
  | {
      type: 'information'
      data: { artists: MusicCredit[]; albumReference: EntityRef | null; publishedAt: number | null }
    }
  | { type: 'categories'; data: { name: string; group: string }[] }
  | { type: 'write'; data: WriteImpact }
export interface SourceDescriptor {
  source: string
  displayName: string
  contractVersion: number
  coverHosts?: string[]
  shareLinkTemplates?: Partial<Record<EntityKind, string>>
  capabilities: string[]
}
export interface MusicSourceClient {
  sources(): Promise<SourceDescriptor[]>
  getTrack(reference: EntityRef): Promise<MusicTrack>
  query(
    source: string,
    request: MusicReadRequest
  ): Promise<Exclude<MusicResponse, { type: 'write' }>>
  write(
    source: string,
    request: MusicWriteRequest
  ): Promise<Extract<MusicResponse, { type: 'write' }>>
}
export interface LogoutReport {
  localCleared: boolean
  localError: MusicError | null
  remoteError: MusicError | null
}
export type AccountRequest =
  { operation: 'beginLogin' | 'profile' | 'logout' } | { operation: 'pollLogin'; key: string }
export type AccountPresentation =
  | { type: 'challenge'; data: { key: string; image: string } }
  | { type: 'progress'; data: { code: number; message: string } }
  | { type: 'profile'; data: AccountRecord | null }
  | { type: 'logout'; data: LogoutReport }
