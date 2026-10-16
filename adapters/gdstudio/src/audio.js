import { parseFromTokenizer } from 'music-metadata'
import { fromBuffer } from 'strtok3/core'
import { fail } from './protocol.js'

// Random access lets the mature parser skip artwork/media blocks and read a
// trailing MP4 moov without downloading the song or allocating its full length.
export async function inspectRemoteAudio(read, expectedDuration, requested, downgrade) {
  const first = await read(0, 65535)
  const size = first.size
  if (
    !Number.isSafeInteger(size) ||
    size <= 512 ||
    size > 512 * 1024 * 1024 ||
    first.bytes.length > 65536 ||
    first.bytes.length > size
  )
    fail('invalidData', 'probe-size')
  const chunks = [{ start: 0, bytes: first.bytes }]
  const tokenizer = fromBuffer(first.bytes, { maxBufferSize: 65536 })
  tokenizer.fileInfo.size = size
  let transferred = first.bytes.length
  let requests = 1
  tokenizer.peekBuffer = async (target, options = {}) => {
    const { position, length, mayBeLess } = tokenizer.normalizeOptions(target, options)
    const count = Math.min(length, size - position)
    if (position > size || count < 0 || (!mayBeLess && count < length) || length > 65536)
      fail('invalidData', 'probe-range')
    if (!count) return 0
    let chunk = chunks.find(
      (item) => position >= item.start && position + count <= item.start + item.bytes.length
    )
    if (!chunk) {
      const start = Math.min(position, Math.max(0, size - 65536))
      const end = Math.min(size - 1, Math.max(position + count - 1, start + 65535))
      if (++requests > 8 || transferred + end - start + 1 > 256 * 1024)
        fail('invalidData', 'probe-budget')
      const response = await read(start, end)
      if (response.size !== size || response.bytes.length !== end - start + 1)
        fail('invalidData', 'probe-changed')
      chunk = { start, bytes: response.bytes }
      chunks.push(chunk)
      transferred += response.bytes.length
    }
    target.set(chunk.bytes.subarray(position - chunk.start, position - chunk.start + count))
    return count
  }
  let format
  try {
    ;({ format } = await parseFromTokenizer(tokenizer, {
      skipCovers: true,
      skipPostHeaders: true,
      duration: false
    }))
  } catch (error) {
    if (error.code) throw error
    fail(
      'invalidData',
      `parser-${error.name}${Number.isSafeInteger(error.requestedSize) ? `-${error.requestedSize}` : ''}`
    )
  }
  if (
    !Number.isFinite(format.duration) ||
    format.duration <= 0 ||
    !Number.isFinite(format.bitrate) ||
    format.bitrate <= 0 ||
    !/^(MPEG|FLAC|(?:M4A|M4B|MP4|isom|iso2|M4V)(?:\/[A-Za-z0-9 ]+)*)(?: )?$/.test(
      format.container ?? ''
    ) ||
    !/Layer 3|FLAC|AAC|ALAC/.test(format.codec ?? '')
  )
    fail(
      'invalidData',
      `audio-${format.container}-${format.codec}-${format.duration ? 'duration' : 'no-duration'}`.slice(
        0,
        120
      )
    )
  if (
    !Number.isFinite(expectedDuration) ||
    expectedDuration <= 0 ||
    Math.abs(format.duration * 1000 - expectedDuration) > Math.max(2000, expectedDuration * 0.01)
  )
    fail('permissionDenied', 'audio-duration')
  const actual = format.lossless
    ? format.bitsPerSample > 16 || format.sampleRate > 48000
      ? 'hires'
      : 'lossless'
    : `${Math.round(format.bitrate / 1000)}k`
  const rank = (quality) =>
    quality === 'hires' ? 10001 : quality === 'lossless' ? 10000 : Number.parseInt(quality)
  if (!downgrade && rank(actual) + (format.lossless ? 0 : 12) < rank(requested))
    fail('permissionDenied', 'audio-quality')
  return {
    metadata: { actualQuality: actual, extent: { kind: 'full' }, expiresAtMs: null },
    format,
    bytesRead: transferred
  }
}
