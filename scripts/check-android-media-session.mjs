import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'

// Existing queue only. Native media keys drive control; WebView IPC only observes state.
// Does not print titles, URLs, account data or queue contents. Ends paused.
const root = fileURLToPath(new URL('..', import.meta.url))
const adb = resolve(root, '.local/android/sdk/platform-tools/adb.exe')
const shell = (...args) => execFileSync(adb, ['shell', ...args], { encoding: 'utf8', cwd: root })
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
const pid = shell('pidof', 'com.miaoyww.nons').trim()
assert.match(pid, /^\d+$/, 'open the debug APK on one connected device first')
execFileSync(adb, ['forward', 'tcp:9228', `localabstract:webview_devtools_remote_${pid}`])
const pages = await fetch('http://127.0.0.1:9228/json', { signal: AbortSignal.timeout(5000) }).then(
  (r) => r.json()
)
const page = pages.find((p) => p.url.includes('tauri.localhost'))
assert.ok(page, 'debug WebView is required for observing the Rust core')
const socket = new WebSocket(page.webSocketDebuggerUrl)
await new Promise((resolve, reject) => {
  socket.addEventListener('open', resolve, { once: true })
  socket.addEventListener('error', reject, { once: true })
})
let sequence = 0
const pending = new Map()
socket.addEventListener('message', ({ data }) => {
  const reply = JSON.parse(data)
  pending.get(reply.id)?.(reply)
  pending.delete(reply.id)
})
async function call(command, args = {}) {
  const id = ++sequence
  let timeout
  const response = new Promise((resolve, reject) => {
    pending.set(id, resolve)
    timeout = setTimeout(() => {
      pending.delete(id)
      reject(Error('native observation timed out'))
    }, 10000)
  })
  socket.send(
    JSON.stringify({
      id,
      method: 'Runtime.evaluate',
      params: {
        expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`,
        awaitPromise: true,
        returnByValue: true
      }
    })
  )
  try {
    const reply = await response
    assert.ok(!reply.error && !reply.result?.exceptionDetails, 'native observation failed')
    return reply.result.result.value
  } finally {
    clearTimeout(timeout)
  }
}
async function waitFor(predicate) {
  for (let i = 0; i < 40; i++) {
    const snapshot = await call('player_snapshot')
    assert.equal(snapshot.state.error, null, 'Rust playback failed')
    assert.equal(snapshot.state.mediaError, null, 'native media bridge failed')
    if (predicate(snapshot)) return snapshot
    await sleep(250)
  }
  throw Error('native media command did not reach the expected playback state')
}
const dispatch = (key) => shell('cmd', 'media_session', 'dispatch', key)
try {
  const restored = await call('player_snapshot')
  assert.ok(
    restored.state.index !== null && restored.queue.queue.length >= 2,
    'prepare a playable queue with at least two tracks'
  )
  await call('player_action', { action: 'resume' }) // Bootstrap an active session once.
  await waitFor((s) => s.state.status === 'playing')
  assert.ok(
    shell('cmd', 'media_session', 'list-sessions').includes('com.miaoyww.nons'),
    'Nons session not registered'
  )
  dispatch('pause')
  const paused = await waitFor((s) => s.state.status === 'paused')
  await sleep(750)
  const still = await call('player_snapshot')
  assert.equal(still.progress.positionMs, paused.progress.positionMs, 'pause position advanced')
  assert.ok(
    !shell('dumpsys', 'power').match(/PARTIAL_WAKE_LOCK\s+.*Nons:playback/),
    'pause leaked the CPU wake lock'
  )
  dispatch('play')
  const playing = await waitFor((s) => s.state.status === 'playing')
  const index = playing.state.index
  dispatch('next')
  await waitFor((s) => s.state.status === 'playing' && s.state.index !== index)
  dispatch('previous')
  await waitFor((s) => s.state.status === 'playing' && s.state.index === index)
  dispatch('headsethook')
  await waitFor((s) => s.state.status === 'paused')
  dispatch('play')
  await waitFor((s) => s.state.status === 'playing')
  shell('input', 'keyevent', 'KEYCODE_HOME')
  dispatch('pause')
  await waitFor((s) => s.state.status === 'paused')
  dispatch('play')
  const background = await waitFor((s) => s.state.status === 'playing')
  await sleep(1500)
  const advancing = await call('player_snapshot')
  assert.ok(
    advancing.progress.positionMs > background.progress.positionMs + 500,
    'background playback did not advance'
  )
  dispatch('pause')
  await waitFor((s) => s.state.status === 'paused')
  console.log(
    'PASS: native MediaSession play/pause/next/previous/headset key/background control; paused CPU wake lock released'
  )
} finally {
  try {
    await call('player_action', { action: 'pause' })
  } catch {}
  socket.close()
  execFileSync(adb, ['forward', '--remove', 'tcp:9228'])
}
