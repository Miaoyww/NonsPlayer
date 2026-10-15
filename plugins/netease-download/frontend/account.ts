import type { PluginClient } from '@app/plugin-sdk'
type Chunk = { data: string; offset: number; total: number; generation: number; scope: string }
export async function accountCredential(client: PluginClient) {
  const first = await client.call<Chunk>('accounts.credential-read', {
    source: 'netease',
    offset: 0,
    length: 4096
  })
  if (!Number.isSafeInteger(first.total) || first.total < 0 || first.total > 65536)
    throw new Error('凭据大小无效')
  const bytes = new Uint8Array(first.total)
  let offset = 0
  let chunk = first
  while (offset < bytes.length) {
    const data = Uint8Array.from(atob(chunk.data), (value) => value.charCodeAt(0))
    if (
      chunk.offset !== offset ||
      chunk.total !== first.total ||
      chunk.scope !== first.scope ||
      data.length === 0 ||
      offset + data.length > bytes.length
    )
      throw new Error('账号上下文或凭据分块已变化')
    bytes.set(data, offset)
    offset += data.length
    if (offset < bytes.length)
      chunk = await client.call<Chunk>('accounts.credential-read', {
        source: 'netease',
        offset,
        length: 4096,
        scope: first.scope
      })
  }
  // This platform-specific plugin owns its credential format interpretation.
  try {
    return {
      cookie: bytes.length ? new TextDecoder('utf-8', { fatal: true }).decode(bytes) : null,
      generation: first.generation,
      scope: first.scope
    }
  } finally {
    bytes.fill(0)
  }
}
export const accountScope = (client: PluginClient) =>
  client.call<Chunk>('accounts.credential-read', { source: 'netease', offset: 0, length: 1 })
