import { adapterMusicu } from 'qq-protocol/auth/qrLogin.js'
import { call as hostCall } from 'nons:plugin/host@1.0.0'
import { authHttp, authService, session, authenticated } from './auth.js'
import { hostFetch, drive } from './runtime/platform.js'
import {
  track,
  text,
  entity,
  reference,
  checkReference,
  pageState,
  pageResult,
  accountRecord,
  fail
} from './model.js'

export async function rpc(module, method, param, requiresAuth = false) {
  if (requiresAuth) {
    const auth = await session()
    if (!auth) fail('unauthenticated')
    return authenticated(() => adapterMusicu(authHttp, auth, module, method, param))
  }
  const request = { module, method, param }
  const send = async (comm, target = request) => {
    const response = await hostFetch('https://u.y.qq.com/cgi-bin/musicu.fcg', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Cookie: 'tmeLoginType=-1;',
        'User-Agent': 'okhttp/3.14.9'
      },
      body: JSON.stringify({ comm, request: target })
    })
    if (!response.ok) fail(response.status === 429 ? 'rateLimited' : 'network')
    return response.json()
  }
  let value = await send({
    ct: 11,
    cv: '1003006',
    v: '1003006',
    tmeAppID: 'qqmusiclight',
    uid: '0',
    udid: '0'
  })
  let desktopSearch = false
  if (
    module === 'music.search.SearchCgiService' &&
    method === 'DoSearchForQQMusicLite' &&
    (value.code === 2001 || (value.code === 0 && value.request?.code === 2001))
  ) {
    // Lite can reject both client contexts with 2001. Try the public desktop
    // search once, retaining the keyword, category and page within this deadline.
    try {
      hostCall(
        'diagnostic.report',
        JSON.stringify({
          stage: 'search.retry-desktop',
          class: 'UpstreamRejected',
          code: 'upstream-2001'
        })
      )
    } catch {}
    desktopSearch = true
    value = await send(undefined, {
      module,
      method: 'DoSearchForQQMusicDesktop',
      param: {
        query: param.query,
        search_type: param.search_type,
        num_per_page: param.num_per_page,
        page_num: param.page_num
      }
    })
  }
  // This endpoint returns request code 500 plus an empty track for absent numeric IDs.
  if (
    method === 'get_song_detail_yqq' &&
    value.code === 0 &&
    value.request?.code === 500 &&
    !value.request?.data?.track_info?.mid
  )
    fail('notFound', 'track.detail-not-found')
  if (value.code !== 0 || value.request?.code !== 0) {
    const code = value.request?.code ?? value.code
    fail('sourceUnavailable', Number.isSafeInteger(code) ? `rpc.upstream-${code}` : 'rpc.upstream')
  }
  const data = value.request.data
  if (!desktopSearch) return data
  const keys = [
    ['song', 'item_song'],
    ['singer', 'singer'],
    ['album', 'item_album'],
    ['songlist', 'item_songlist']
  ][param.search_type]
  const items = keys && data?.body?.[keys[0]]?.list
  if (!Array.isArray(items)) fail('invalidData', 'search.desktop-data')
  return {
    ...data,
    body: {
      [keys[1]]: items.map((item) => ({
        ...item,
        albumMid: item.albumMID ?? item.albumMid,
        picurl: item.imgurl ?? item.picurl
      }))
    }
  }
}
async function legacy(path, params) {
  const url = new URL(path, 'https://c.y.qq.com')
  url.search = new URLSearchParams({ format: 'json', outCharset: 'utf-8', ...params })
  const response = await hostFetch(url, { headers: { Referer: 'https://y.qq.com/' } })
  if (!response.ok) fail('network')
  return response.json()
}
async function album(id) {
  const value = await legacy('/v8/fcg-bin/fcg_v8_album_info_cp.fcg', { albummid: id })
  if (value.code !== 0 || !value.data) fail('notFound')
  return value.data
}
const artistSongs = (id, offset, limit) =>
  rpc('music.web_singer_info_svr', 'get_singer_detail_info', {
    sort: 5,
    singermid: id,
    sin: offset,
    num: limit
  })
async function collections(auth, filter) {
  const created = await rpc(
    'music.musicasset.PlaylistBaseRead',
    'GetPlaylistByUin',
    { uin: auth.credential.str_musicid || String(auth.credential.musicid) },
    true
  )
  const owner =
    text(auth.credential.nick) ||
    accountRecord(await authenticated(() => authService.getUserDetail('current')), auth.credential)
      .displayName
  let values = Array.isArray(created.v_playlist)
    ? created.v_playlist.map((value) => ({
        ...value,
        creatorName: text(value.creatorName) || owner,
        ownerAccount: accountRecord({}, auth.credential).reference
      }))
    : []
  values = values.filter((value) => Number(value.dirId ?? value.dirid) !== 201)
  if (filter === 'created') return values
  const encrypted = auth.credential.encryptUin
  const favorites = []
  if (encrypted) {
    for (let offset = 0; offset < 1000;) {
      const data = await rpc(
        'music.musicasset.PlaylistFavRead',
        'CgiGetPlaylistFavInfo',
        { uin: encrypted, offset, size: 100 },
        true
      )
      const page = Array.isArray(data.v_list) ? data.v_list : []
      favorites.push(...page)
      offset += page.length
      if (!page.length || !(data.hasmore === true || data.hasmore === 1)) break
    }
  }
  if (filter === 'collected') return favorites
  const seen = new Set()
  return [...values, ...favorites].filter((value) => {
    const id = String(value.tid ?? value.dissid ?? value.disstid ?? '')
    if (!id || seen.has(id)) return false
    seen.add(id)
    return true
  })
}
export async function song(mid) {
  const numeric = mid.startsWith('id:')
  const songId = numeric ? Number(mid.slice(3)) : null
  if (numeric && (!Number.isSafeInteger(songId) || songId <= 0))
    fail('invalidData', 'track.numeric-id')
  const data = await rpc('music.pf_song_detail_svr', 'get_song_detail_yqq', {
    song_type: 0,
    song_mid: numeric ? '' : mid,
    song_id: numeric ? songId : ''
  })
  const value = data.track_info
  if (!value || (!numeric && value.mid !== mid) || (numeric && Number(value.id) !== songId))
    fail('notFound', 'track.detail-identity')
  return value
}
export async function readTrack(value) {
  const id = checkReference(value, 'track')
  const result = track(await song(id))
  // Requests for retained numeric identities must not change their queue identity.
  return { ...result, reference: value }
}
async function tracksResponse(values, state, total, description = null) {
  const repaired = new Map()
  const items = []
  for (const value of values.slice(0, state.limit)) {
    try {
      items.push(track(value))
    } catch (error) {
      if (error?.diagnostics?.failureStage !== 'track.mid') throw error
      const original = value.songInfo ?? value.musicData ?? value
      const id = Number(original.id ?? original.songid)
      if (!Number.isSafeInteger(id) || id <= 0) throw error
      if (!repaired.has(id)) {
        try {
          repaired.set(id, track(await song(`id:${id}`)))
        } catch (failure) {
          if (failure?.code !== 'notFound' && failure?.diagnostics?.failureStage !== 'track.mid')
            throw failure
          repaired.set(id, null)
        }
      }
      items.push(repaired.get(id) ?? track(original, 'notFound'))
    }
  }
  if (items.some((item) => item.unavailableReason)) {
    try {
      hostCall(
        'diagnostic.report',
        JSON.stringify({ stage: 'track.mid-unavailable', class: 'MissingMid', code: 'notFound' })
      )
    } catch {} // Diagnostics must not discard an otherwise usable catalog page.
  }
  return {
    type: 'tracks',
    data: {
      ...pageResult(items, state, Number.isFinite(total) ? total : undefined),
      total: Number.isFinite(total) ? total : null,
      description
    }
  }
}
async function playlist(id, state) {
  const auth = await session()
  if (id === 'liked') {
    const data = await authenticated(() =>
      authService.getUserLikedSongs('current', state.offset, state.limit)
    )
    return {
      data: { ...data, disstid: id, dissname: '我喜欢' },
      response: () => tracksResponse(data.songlist ?? [], state, data.total_song_num)
    }
  }
  if (!/^\d+$/.test(id) || !Number.isSafeInteger(Number(id)))
    fail('invalidData', 'playlist.numeric-id')
  if (auth) {
    const created = await collections(auth, 'created')
    const own = created.find((value) => String(value.tid ?? value.disstid ?? value.dissid) === id)
    if (own) {
      const data = await authenticated(() =>
        authService.getOwnedPlaylistSongs('current', {
          disstid: Number(id),
          dirid: Number(own.dirId ?? own.dirid ?? 0),
          offset: state.offset,
          limit: state.limit
        })
      )
      return {
        data: { ...own, ...data },
        response: () =>
          tracksResponse(data.songlist ?? [], state, data.total_song_num, data.desc ?? null)
      }
    }
  }
  // The legacy CGI returns the entire playlist even for a single requested page.
  // Use the paginated protocol for public and collected playlists as well.
  const data = await rpc(
    'music.srfDissInfo.aiDissInfo',
    'uniform_get_Dissinfo',
    {
      disstid: Number(id),
      userinfo: 1,
      tag: 1,
      orderlist: 1,
      song_begin: state.offset,
      song_num: state.limit,
      onlysonglist: 0,
      enc_host_uin: ''
    },
    !!auth
  )
  if (data.code && data.code !== 0) fail('sourceUnavailable', 'playlist.upstream-result')
  const values = data.songlist ?? []
  // Some generated playlists ignore the requested page and return all songs.
  const total = Number(data.total_song_num)
  const selected =
    values.length > state.limit ? values.slice(state.offset, state.offset + state.limit) : values
  const metadata = { ...data.dirinfo, ...data }
  return {
    data: metadata,
    response: () => tracksResponse(selected, state, total, metadata.desc ?? null)
  }
}
export async function business(request) {
  if (request.operation === 'search') {
    const kinds = {
      track: [0, 'item_song'],
      artist: [1, 'singer'],
      album: [2, 'item_album'],
      playlist: [3, 'item_songlist']
    }
    const selected = kinds[request.kind]
    if (!selected) fail('invalidData')
    const state = pageState(request.page, JSON.stringify(['search', request.kind, request.keyword]))
    const data = await rpc('music.search.SearchCgiService', 'DoSearchForQQMusicLite', {
      query: request.keyword,
      search_type: selected[0],
      num_per_page: state.limit,
      page_num: Math.floor(state.offset / state.limit) + 1,
      highlight: 0,
      nqc_flag: 0,
      page_id: 1,
      grp: 1,
      remoteplace: 'search.android.keyboard',
      search_id: globalThis.crypto.randomUUID().replaceAll('-', '')
    })
    const values = data.body?.[selected[1]] ?? []
    return request.kind === 'track'
      ? tracksResponse(values, state, data.meta?.sum)
      : {
          type: 'entities',
          data: pageResult(
            values.slice(0, state.limit).map((value) => entity(value, request.kind)),
            state,
            data.meta?.sum
          )
        }
  }
  if (request.operation === 'trackInformation') {
    const value = await readTrack(request.track)
    return {
      type: 'information',
      data: { artists: value.artists, albumReference: value.albumReference, publishedAt: null }
    }
  }
  if (request.operation === 'tracks' && request.entity.kind === 'playlist') {
    const id = checkReference(request.entity, 'playlist')
    return (
      await playlist(id, pageState(request.page, JSON.stringify(['playlist', id])))
    ).response()
  }
  if (['detail', 'tracks'].includes(request.operation) && request.entity.kind === 'album') {
    const id = checkReference(request.entity, 'album')
    const data = await album(id)
    if (request.operation === 'detail')
      return {
        type: 'detail',
        data: { item: entity(data, 'album'), description: data.desc ?? null, albumCount: null }
      }
    const state = pageState(request.page, JSON.stringify(['album', id]))
    const values = data.list ?? []
    return tracksResponse(
      values.slice(state.offset, state.offset + state.limit),
      state,
      values.length
    )
  }
  if (
    (request.operation === 'detail' && request.entity.kind === 'artist') ||
    ['artistTracks', 'artistAlbums'].includes(request.operation)
  ) {
    const id = checkReference(request.entity ?? request.artist, 'artist')
    const state = pageState(request.page, JSON.stringify([request.operation, id]))
    if (request.operation === 'artistAlbums') {
      const data = await rpc('music.musichallAlbum.AlbumListServer', 'GetAlbumList', {
        sort: 5,
        singermid: id,
        begin: state.offset,
        num: state.limit
      })
      return {
        type: 'entities',
        data: pageResult(
          (data.albumList ?? []).slice(0, state.limit).map((value) => entity(value, 'album')),
          state,
          data.total
        )
      }
    }
    const data = await artistSongs(
      id,
      state.offset,
      request.operation === 'detail' ? 1 : state.limit
    )
    if (request.operation === 'detail')
      return {
        type: 'detail',
        data: {
          item: entity({ ...data.singer_info, mid: id, total_song: data.total_song }, 'artist'),
          description: data.singer_brief ?? null,
          albumCount: data.total_album ?? null
        }
      }
    return tracksResponse(data.songlist ?? [], state, data.total_song)
  }
  if (request.operation === 'detail' && request.entity.kind === 'playlist') {
    const id = checkReference(request.entity, 'playlist')
    const { data } = await playlist(id, pageState({ limit: 1 }, JSON.stringify(['playlist', id])))
    return {
      type: 'detail',
      data: {
        item: entity({ ...data, disstid: id }, 'playlist'),
        description: data.desc ?? null,
        albumCount: null
      }
    }
  }
  if (request.operation === 'librarySummary') {
    const auth = await session()
    if (!auth) fail('unauthenticated')
    const profile = await authenticated(() => authService.getUserDetail('current'))
    let likedTracks = [],
      likedError = null,
      likedPlaylist = null
    try {
      const data = await authenticated(() => authService.getUserLikedSongs('current', 0, 25))
      likedTracks = (
        await tracksResponse(data?.songlist ?? [], { limit: 25, offset: 0 }, data.total_song_num)
      ).data.items
      likedPlaylist = entity(
        { disstid: 'liked', dissname: '我喜欢', songnum: data.total_song_num },
        'playlist'
      )
    } catch {
      likedError = { code: 'sourceUnavailable', retryAfterMs: null }
    }
    return {
      type: 'summary',
      data: {
        account: accountRecord(profile, auth.credential),
        likedPlaylist,
        likedTracks,
        likedError
      }
    }
  }
  if (request.operation === 'favoritesPage') {
    const state = pageState(request.page, JSON.stringify(['playlist', 'liked']))
    const result = await playlist('liked', state)
    const page = (await result.response()).data
    return {
      type: 'favoritesPage',
      data: { items: page.items.map((item) => item.reference), nextCursor: page.nextCursor }
    }
  }
  if (request.operation === 'libraryCollections') {
    const auth = await session()
    if (!auth) fail('unauthenticated')
    const state = pageState(request.page, JSON.stringify(['library', request.kind, request.filter]))
    if (request.kind === 'album') {
      const data = await authenticated(() =>
        authService.getUserAlbums('current', state.offset, state.limit)
      )
      const values = data?.albumlist ?? data?.list ?? []
      return {
        type: 'entities',
        data: pageResult(
          values.slice(0, state.limit).map((value) => entity(value, 'album')),
          state,
          data?.totalalbum
        )
      }
    }
    if (request.kind === 'playlist') {
      const values = await collections(auth, request.filter)
      return {
        type: 'entities',
        data: pageResult(
          values
            .slice(state.offset, state.offset + state.limit)
            .map((value) => entity(value, 'playlist')),
          state,
          values.length
        )
      }
    }
    fail('unsupported')
  }
  fail('unsupported')
}

const qualityFiles = {
  standard: { value: 128, prefix: 'M500', extension: '.mp3', size: 'size_128mp3' },
  exhigh: { value: 320, prefix: 'M800', extension: '.mp3', size: 'size_320mp3' },
  lossless: { value: 'flac', prefix: 'F000', extension: '.flac', size: 'size_flac' }
}
export async function resolvePlayback(request) {
  const identifier = checkReference(request.track, 'track')
  if (!(await session())) fail('unauthenticated')
  const value = await song(identifier)
  const mid = text(value.mid) || text(value.songmid)
  if (!mid) fail('notFound', 'playback.track-mid')
  const order = ['hires', 'lossless', 'exhigh', 'higher', 'standard']
  const start = order.indexOf(request.preferredQuality)
  if (start < 0) fail('invalidData')
  if (!request.allowDowngrade && !qualityFiles[request.preferredQuality]) fail('unsupported')
  const candidates = (
    request.allowDowngrade ? order.slice(start) : [request.preferredQuality]
  ).filter((quality) => qualityFiles[quality])
  let rejectedAt = 'playback.file-size'
  let rejectedCode = 'permissionDenied'
  for (const quality of candidates) {
    const file = qualityFiles[quality]
    const size = Number(value.file?.[file.size])
    if (!Number.isSafeInteger(size) || size <= 0) continue
    const media = value.file?.media_mid
    if (!media || !/^[a-zA-Z0-9]+$/.test(media)) fail('invalidData')
    const result = await authenticated(() =>
      authService.getMusicPlay('current', mid, file.value, media)
    )
    const url = result?.[mid]?.url
    if (!url) {
      rejectedAt = 'playback.url-missing'
      rejectedCode = 'sourceUnavailable'
      continue
    }
    const parsed = new URL(url)
    // Vkey binds the authorization to the requested song. The CDN may canonicalize
    // its basename; classify the returned format, then verify its full catalog size.
    const filename = parsed.pathname.split('/').pop() ?? ''
    if (!filename.startsWith(file.prefix) || !filename.endsWith(file.extension)) {
      rejectedAt = 'playback.filename'
      rejectedCode = 'invalidData'
      continue
    }
    const response = await hostFetch(url, {
      method: 'HEAD',
      headers: { 'Accept-Encoding': 'identity' }
    })
    if (response.status !== 200) {
      rejectedAt = `playback.http-${response.status}`
      rejectedCode =
        response.status === 401
          ? 'unauthenticated'
          : response.status === 403
            ? 'permissionDenied'
            : response.status === 404
              ? 'notFound'
              : response.status === 429
                ? 'rateLimited'
                : 'sourceUnavailable'
      continue
    }
    if (Number(response.headers.get('content-length')) !== size) {
      rejectedAt = 'playback.content-length'
      rejectedCode = 'invalidData'
      continue
    }
    return {
      metadata: { actualQuality: quality, extent: { kind: 'full' }, expiresAtMs: null },
      url,
      headers: []
    }
  }
  fail(rejectedCode, rejectedAt)
}
