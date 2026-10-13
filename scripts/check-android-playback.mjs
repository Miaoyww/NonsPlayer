import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'

// Run against a connected, unlocked device with an existing online playback queue.
// Does not log account data, URLs, or the queue. Leaves playback paused.
const root = fileURLToPath(new URL('..', import.meta.url))
const adb = resolve(root, '.local/android/sdk/platform-tools/adb.exe')
const switches = Number(process.argv[2] ?? 12)
const backgroundSeconds = Number(process.argv[3] ?? 15)
assert.ok(Number.isInteger(switches) && switches >= 1 && switches <= 100)
assert.ok(Number.isInteger(backgroundSeconds) && backgroundSeconds >= 1 && backgroundSeconds <= 60)
const shell = (...args) => execFileSync(adb, ['shell', ...args], { encoding: 'utf8', cwd: root })
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms))
const pid = shell('pidof', 'com.miaoyww.nons').trim()
assert.match(pid, /^\d+$/, 'start Nons on one connected device before running')
execFileSync(adb, ['forward', 'tcp:9227', `localabstract:webview_devtools_remote_${pid}`])
const pages = await fetch('http://127.0.0.1:9227/json', { signal: AbortSignal.timeout(5000) }).then(
  (r) => r.json()
)
const page = pages.find((p) => p.url.includes('tauri.localhost'))
assert.ok(page, 'debug APK WebView must be available')
const socket = new WebSocket(page.webSocketDebuggerUrl)
await new Promise((resolve, reject) => {
  socket.addEventListener('open', resolve, { once: true })
  socket.addEventListener('error', reject, { once: true })
})
let id = 0
const pending = new Map()
socket.addEventListener('message', ({ data }) => {
  const reply = JSON.parse(data)
  pending.get(reply.id)?.(reply)
  pending.delete(reply.id)
})
async function call(command, args = {}) {
  const next = ++id
  const result = new Promise((resolve) => pending.set(next, resolve))
  socket.send(
    JSON.stringify({
      id: next,
      method: 'Runtime.evaluate',
      params: {
        expression: `window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(args)})`,
        awaitPromise: true,
        returnByValue: true
      }
    })
  )
  const reply = await Promise.race([
    result,
    sleep(10000).then(() => {
      throw Error('IPC timeout')
    })
  ])
  assert.ok(!reply.error && !reply.result?.exceptionDetails, 'native command failed')
  return reply.result.result.value
}
async function advancing(index, interrupt) {
  await call('player_jump', { index })
  if (interrupt) {
    await call('player_action', { action: 'pause' })
    await sleep(100)
    await call('player_action', { action: 'resume' })
  }
  let ready
  for (let attempt = 0; attempt < 30; attempt++) {
    await sleep(500)
    ready = await call('player_snapshot')
    assert.equal(ready.state.error, null, 'playback error')
    if (ready.state.status === 'playing' && ready.progress.positionMs > 200) break
  }
  assert.equal(ready.state.index, index, 'another playback control changed the track')
  assert.equal(ready.state.status, 'playing', 'track did not start')
  await sleep(2000)
  const after = await call('player_snapshot')
  assert.ok(
    after.progress.positionMs - ready.progress.positionMs > 1200,
    'playback stuck near 00:00'
  )
  return after
}
const since = shell('date', "'+%m-%d %H:%M:%S.000'").trim()
let screenOff = false
try {
  const initial = await call('player_snapshot')
  assert.ok(initial.queue.queue.length, 'queue must contain playable tracks')
  const count = initial.queue.queue.length
  for (let turn = 0; turn < switches; turn++) {
    const index = (count - Math.min(count, 6) + (turn % Math.min(count, 6))) % count
    const state = await advancing(index, turn % 4 === 3)
    console.log(JSON.stringify({ switch: turn + 1, index, positionMs: state.progress.positionMs }))
  }
  const before = await call('player_snapshot')
  shell('input', 'keyevent', '3')
  shell('input', 'keyevent', '223')
  screenOff = true
  await sleep(backgroundSeconds * 1000)
  const service = shell('dumpsys', 'activity', 'services', 'com.miaoyww.nons')
  assert.ok(
    service.includes('isForeground=true'),
    'background playback needs its foreground service'
  )
  shell('input', 'keyevent', '224')
  screenOff = false
  shell('am', 'start', '-n', 'com.miaoyww.nons/.MainActivity')
  const after = await call('player_snapshot')
  const advance = after.progress.positionMs - before.progress.positionMs
  assert.ok(advance > (backgroundSeconds - 2) * 1000, 'screen-off playback stalled')
  const log = execFileSync(adb, ['logcat', '-d', `--pid=${pid}`, '-T', since], {
    encoding: 'utf8',
    maxBuffer: 8 * 1024 * 1024
  })
  const skew = (log.match(/correct clock skew/g) ?? []).length
  const starvation = (log.match(/No segment available/g) ?? []).length
  console.log(JSON.stringify({ backgroundAdvanceMs: advance, skew, starvation }))
  assert.equal(starvation, 0, 'OpenSL ES output ran out of segments')
  assert.equal(skew, 0, 'output repeatedly inserted/dropped samples for clock skew')
  await call('player_action', { action: 'pause' })
  await sleep(1000)
  assert.ok(
    !shell('dumpsys', 'activity', 'services', 'com.miaoyww.nons').includes('isForeground=true'),
    'pause must stop the service'
  )
  const locks = shell('dumpsys', 'power').match(/(?:^|\n)Wake Locks:[\s\S]*?(?=\n\S|$)/)?.[0] ?? ''
  assert.ok(!locks.includes('Nons:playback'), 'pause must release the active CPU wake lock')
  console.log('PASS: switching, background/screen-off playback and pause cleanup')
} finally {
  if (screenOff) shell('input', 'keyevent', '224')
  shell('am', 'start', '-n', 'com.miaoyww.nons/.MainActivity')
  await call('player_action', { action: 'pause' }).catch(() => {})
  socket.close()
}
