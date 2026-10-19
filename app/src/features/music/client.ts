import { nativeCall, scopedNativeCall } from '@/lib/player'
import type { Track } from '@/lib/player'
import { invalidateNativeCache } from '@/lib/runtime-cache'
import type {
  AccountPresentation,
  AccountRecord,
  AccountRef,
  AccountRequest,
  EntityRef,
  MusicReadRequest,
  MusicResponse,
  MusicSourceClient,
  MusicTrack,
  MusicWriteRequest,
  SourceDescriptor
} from './types'

async function scoped<T>(
  source: string,
  command: string,
  args: Record<string, unknown>,
  refresh = false
): Promise<T> {
  return scopedNativeCall<T>(source, command, { ...args, refresh })
}
export const musicClient: MusicSourceClient & {
  sources(): Promise<SourceDescriptor[]>
  queueTrack(reference: EntityRef): Promise<Track>
  queueTracks(source: string, tracks: MusicTrack[]): Promise<Track[]>
  storedTracks(source: string, tracks: MusicTrack[]): Promise<Track[]>
  accounts(source: string): Promise<AccountRecord[]>
  currentAccount(source: string): Promise<AccountRecord | null>
  account(source: string, request: AccountRequest): Promise<AccountPresentation>
  selectAccount(reference: AccountRef): Promise<void>
  cancelLogin(source: string, key: string): Promise<void>
  removeAccount(reference: AccountRef): Promise<void>
  refresh(
    source: string,
    request: MusicReadRequest
  ): Promise<Exclude<MusicResponse, { type: 'write' }>>
} = {
  sources: () => nativeCall('music_sources'),
  queueTrack: (reference) => scoped(reference.source, 'music_queue_track', { reference }),
  queueTracks: (source, tracks) => scoped(source, 'music_queue_tracks', { tracks }),
  storedTracks: (source, tracks) => scoped(source, 'music_stored_tracks', { tracks }),
  getTrack: (reference: EntityRef) =>
    scoped<MusicTrack>(reference.source, 'music_read_track', { reference }),
  query: (source, request) => scoped(source, 'music_query', { request }),
  refresh: (source, request) => scoped(source, 'music_query', { request }, true),
  async write(source: string, request: MusicWriteRequest) {
    const response = await nativeCall<Extract<MusicResponse, { type: 'write' }>>('music_write', {
      source,
      request
    })
    invalidateNativeCache()
    return response
  },
  accounts: (source) => nativeCall('music_accounts', { source }),
  currentAccount: (source) => nativeCall('music_current_account', { source }),
  async account(source, request) {
    const result = await nativeCall<AccountPresentation>('music_account', { source, request })
    if (request.operation === 'logout' || (result.type === 'progress' && result.data.code === 803))
      invalidateNativeCache(undefined, 'context')
    return result
  },
  selectAccount: (reference) => nativeCall('music_select_account', { reference }),
  cancelLogin: (source, key) => nativeCall('music_cancel_login', { source, key }),
  removeAccount: (reference) => nativeCall('music_remove_account', { reference })
}
