// Extract only fixed categories and numeric result codes before the protocol library
// reduces each Vkey entry to { url, error }. Never return payloads or identifiers.
export function playbackDiagnostics(body, request) {
  const rpc = request?.req_0
  if (rpc?.module !== 'music.vkey.GetVkey' || rpc.method !== 'UrlGetVkey') return []
  const numericCode = (value) => {
    if (typeof value !== 'number' && !(typeof value === 'string' && /^-?[0-9]+$/.test(value)))
      return 'notProvided'
    const number = Number(value)
    return Number.isSafeInteger(number) ? `upstream-${number}` : 'notProvided'
  }
  const rawPrefix = rpc.param?.filename?.[0]?.slice(0, 4)
  const prefix = ['M500', 'M800', 'F000'].includes(rawPrefix) ? rawPrefix : 'unknown'
  const entries = body?.req_0?.data?.midurlinfo
  if (!Array.isArray(entries) || !entries.length)
    return [
      {
        stage: `playback.vkey-entries.${prefix}`,
        class: 'EntriesMissing',
        code: numericCode(body?.req_0?.code ?? body?.code)
      }
    ]
  const diagnostics = []
  for (const entry of entries.slice(0, 8)) {
    if (typeof entry?.purl !== 'string' || !entry.purl)
      diagnostics.push({
        stage: `playback.vkey-empty.${prefix}`,
        class: 'EmptyPurl',
        code: numericCode(entry?.result)
      })
    else if (!rpc.param?.songmid?.includes(String(entry.songmid ?? '').trim()))
      diagnostics.push({
        stage: `playback.vkey-identity.${prefix}`,
        class: 'UnmatchedSong',
        code: numericCode(entry?.result)
      })
  }
  return diagnostics
}
