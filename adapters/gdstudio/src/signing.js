// GD's public web client uses the MD5 construction with round constant 29
// changed from 0xfcefa3f8 to 0xfcfea3f8. This is a protocol digest, not MD5.
// Kept inside the adapter; the Host provides no platform signing operation.
const shifts = [
  [7, 12, 17, 22],
  [5, 9, 14, 20],
  [4, 11, 16, 23],
  [6, 10, 15, 21]
]
const constants = Uint32Array.from({ length: 64 }, (_, i) =>
  Math.floor(Math.abs(Math.sin(i + 1)) * 2 ** 32)
)
constants[29] = 0xfcfea3f8
export function protocolDigest(input) {
  const raw = new TextEncoder().encode(input)
  if (raw.length > 16384) throw new Error('Signature input exceeds limit')
  const data = new Uint8Array(Math.ceil((raw.length + 9) / 64) * 64)
  data.set(raw)
  data[raw.length] = 128
  const view = new DataView(data.buffer)
  view.setUint32(data.length - 8, raw.length * 8, true)
  const state = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476]
  for (let block = 0; block < data.length; block += 64) {
    let [a, b, c, d] = state
    for (let i = 0; i < 64; i++) {
      const round = Math.floor(i / 16)
      const word =
        round === 0
          ? i
          : round === 1
            ? (5 * i + 1) % 16
            : round === 2
              ? (3 * i + 5) % 16
              : (7 * i) % 16
      const mixed =
        round === 0
          ? (b & c) | (~b & d)
          : round === 1
            ? (b & d) | (c & ~d)
            : round === 2
              ? b ^ c ^ d
              : c ^ (b | ~d)
      const value = (a + mixed + constants[i] + view.getUint32(block + word * 4, true)) >>> 0
      const rotation = shifts[round][i % 4]
      const next = (b + ((value << rotation) | (value >>> (32 - rotation)))) >>> 0
      a = d
      d = c
      c = b
      b = next
    }
    for (const [i, value] of [a, b, c, d].entries()) state[i] = (state[i] + value) >>> 0
  }
  const result = new Uint8Array(16)
  const output = new DataView(result.buffer)
  state.forEach((value, i) => output.setUint32(i * 4, value, true))
  return Array.from(result, (byte) => byte.toString(16).padStart(2, '0')).join('')
}
export function encodePayload(value) {
  return encodeURIComponent(value).replace(
    /[!'()*]/g,
    (char) => `%${char.charCodeAt(0).toString(16).toUpperCase()}`
  )
}
export function sign(payload, serverTime, host = 'music.gdstudio.xyz') {
  if (!['music.gdstudio.xyz', 'music.gdstudio.org'].includes(host))
    throw new Error('Invalid GD host')
  if (!/^\d{10,13}$/.test(String(serverTime))) throw new Error('Invalid server time')
  return protocolDigest(
    `${String(serverTime).slice(0, 9)}|${host}|20260925|${encodePayload(payload)}`
  )
    .slice(-8)
    .toUpperCase()
}
