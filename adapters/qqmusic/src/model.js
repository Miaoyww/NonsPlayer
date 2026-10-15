// Platform field interpretation belongs to this package, never to the Host.
export function fail(code, stage = 'catalog.validation') {
  throw Object.assign(new Error(code), { code, diagnostics: { failureStage: stage } })
}
export const text = (value) =>
  typeof value === 'string'
    ? value
    : typeof value === 'number' && Number.isFinite(value)
      ? String(value)
      : ''
export const reference = (kind, id) => ({ source: 'qqmusic', kind, id: text(id) })
export function checkReference(value, kind) {
  if (value?.source !== 'qqmusic' || value.kind !== kind || !/^[a-zA-Z0-9]+$/.test(value.id))
    fail('invalidData', 'reference.source-kind-id')
  return value.id
}
export function artwork(mid, type = 'album') {
  if (!/^[a-zA-Z0-9]+$/.test(text(mid))) return ''
  const prefix = type === 'artist' ? 'T001' : 'T002'
  const url = (size) => `https://y.gtimg.cn/music/photo_new/${prefix}R${size}x${size}M000${mid}.jpg`
  return {
    original: url(800),
    small: url(150),
    normal: url(300),
    large: url(500),
    exlarge: url(800)
  }
}
// When upstream has no resize contract, retain the same original for every semantic size.
export function image(value) {
  const url = text(value)
  if (!url) return ''
  return { original: url, small: url, normal: url, large: url, exlarge: url }
}
export function credits(singers = []) {
  if (!Array.isArray(singers)) return []
  return singers
    .map((value) => ({
      name: text(value.name),
      reference:
        value.mid || value.singer_mid ? reference('artist', value.mid || value.singer_mid) : null
    }))
    .filter((value) => value.name)
}
export function track(value) {
  const song = value.songInfo ?? value.musicData ?? value
  const mid = text(song.mid ?? song.songmid)
  if (!mid) {
    const fields = Object.keys(song)
      .filter((key) => /^[a-zA-Z0-9_-]+$/.test(key))
      .sort()
      .slice(0, 8)
      .map((key) => key.replaceAll('_', '-'))
      .join('.')
    throw Object.assign(new Error('Track MID is missing'), {
      name: `MissingMid.${fields}`.slice(0, 64),
      code: 'invalidData',
      diagnostics: { failureStage: 'track.mid' }
    })
  }
  const album = song.album ?? {}
  const albumMid = text(album.mid ?? song.albummid)
  const artists = credits(song.singer)
  return {
    reference: reference('track', mid),
    title: text(song.title ?? song.name ?? song.songname),
    aliases: song.subtitle ? [text(song.subtitle)] : [],
    artist: artists.map((value) => value.name).join(' / '),
    artists,
    album: text(album.name ?? song.albumname),
    albumReference: albumMid ? reference('album', albumMid) : null,
    durationMs: Math.max(0, Math.round(Number(song.interval ?? 0) * 1000)),
    cover: artwork(albumMid),
    associations: []
  }
}
export function entity(value, kind) {
  const id = text(
    kind === 'playlist'
      ? (value.disstid ?? value.tid ?? value.dissid)
      : (value.mid ??
          value.albumMid ??
          value.albummid ??
          value.singer_mid ??
          value.singerMID ??
          value.singermid)
  )
  if (!id) fail('invalidData', 'entity.id')
  return {
    reference: reference(kind, id),
    name: text(
      value.name ??
        value.title ??
        value.dirName ??
        value.dissname ??
        value.albumName ??
        value.singerName ??
        value.singer_name
    ),
    cover:
      kind === 'playlist'
        ? image(value.bigpicUrl ?? value.picUrl ?? value.logo ?? value.cover ?? value.picurl)
        : artwork(id, kind),
    subtitle:
      kind === 'playlist'
        ? text(
            value.creator?.name ??
              value.creator?.nick ??
              value.creator?.nickname ??
              value.nickname ??
              value.nick ??
              value.creatorName ??
              value.hostname
          )
        : text(value.desc ?? value.subtitle),
    trackCount:
      Math.max(
        0,
        Number(
          value.songnum ??
            value.songNum ??
            value.song_count ??
            value.total_song_num ??
            value.total_song ??
            0
        )
      ) || 0,
    creator: null,
    liked: false,
    playCount: null,
    publishedAt: /^\d{4}-\d{2}-\d{2}$/.test(
      value.publishDate ?? value.publish_date ?? value.aDate ?? ''
    )
      ? Date.parse(value.publishDate ?? value.publish_date ?? value.aDate)
      : null,
    artists: credits(value.singer)
  }
}
// Cursors bind the request identity and page size; they cannot cross query boundaries.
export function pageState(page, scope) {
  const limit = Math.min(25, Math.max(1, Number(page?.limit ?? 25)))
  if (!Number.isInteger(limit)) fail('invalidData', 'page.limit')
  let offset = 0
  if (page?.cursor) {
    let cursor
    try {
      cursor = JSON.parse(page.cursor)
    } catch {
      fail('invalidData', 'page.cursor-json')
    }
    if (
      cursor.scope !== scope ||
      cursor.limit !== limit ||
      !Number.isSafeInteger(cursor.offset) ||
      cursor.offset < 0 ||
      cursor.offset > 100000
    )
      fail('invalidData', 'page.cursor-scope')
    offset = cursor.offset
  }
  return { offset, limit, scope }
}
export function pageResult(items, state, total) {
  const more = Number.isFinite(total)
    ? state.offset + items.length < total
    : items.length === state.limit
  return {
    items,
    nextCursor:
      more && items.length
        ? JSON.stringify({ ...state, offset: state.offset + items.length })
        : null
  }
}
export function accountRecord(profile, credential) {
  const info = profile?.info ?? {}
  const id = text(credential.str_musicid || credential.musicid)
  if (!id) fail('invalidData', 'account.id')
  return {
    reference: { source: 'qqmusic', id },
    displayName:
      text(info.nick ?? info.nickname ?? info.name ?? profile?.nickname ?? credential.nick) || id,
    avatar: image(info.logo ?? info.avatarUrl ?? profile?.avatarUrl ?? credential.logo)
  }
}
