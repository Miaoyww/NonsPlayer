import { test } from 'node:test'
import assert from 'node:assert/strict'
import { inspectRemoteAudio } from '../src/audio.js'

function flac(bits = 16, seconds = 180) {
  const bytes = new Uint8Array(131072)
  bytes.set([0x66, 0x4c, 0x61, 0x43, 0x80, 0, 0, 34])
  const info = new DataView(bytes.buffer, 8, 34)
  info.setUint16(0, 4096)
  info.setUint16(2, 4096)
  const packed =
    (BigInt(44100) << 44n) | (1n << 41n) | (BigInt(bits - 1) << 36n) | BigInt(44100 * seconds)
  info.setBigUint64(10, packed)
  return bytes
}
function reader(bytes, size = bytes.length) {
  return async (start, end) => ({
    bytes: bytes.slice(start, Math.min(end + 1, bytes.length)),
    size
  })
}
test('bounded FLAC reads distinguish lossless and high resolution and reject previews and forbidden downgrades', async () => {
  const standard = await inspectRemoteAudio(reader(flac()), 180000, 'lossless', false)
  assert.equal(standard.metadata.actualQuality, 'lossless')
  assert(standard.bytesRead <= 256 * 1024)
  assert.equal(
    (await inspectRemoteAudio(reader(flac(24)), 180000, 'hires', false)).metadata.actualQuality,
    'hires'
  )
  await assert.rejects(inspectRemoteAudio(reader(flac()), 180000, 'hires', false), {
    code: 'permissionDenied'
  })
  await assert.rejects(inspectRemoteAudio(reader(flac(16, 30)), 180000, 'lossless', true), {
    code: 'permissionDenied'
  })
  await assert.rejects(inspectRemoteAudio(reader(flac()), 0, 'lossless', true), {
    code: 'permissionDenied'
  })
})
test('probe rejects oversized resources and inconsistent range totals', async () => {
  await assert.rejects(
    inspectRemoteAudio(reader(flac(), 513 * 1024 * 1024), 180000, 'lossless', true),
    { code: 'invalidData' }
  )
  let count = 0
  const bytes = flac()
  await assert.rejects(
    inspectRemoteAudio(
      async (start, end) => ({ bytes: bytes.slice(start, end + 1), size: bytes.length + count++ }),
      180000,
      'lossless',
      true
    ),
    { code: 'invalidData' }
  )
})
