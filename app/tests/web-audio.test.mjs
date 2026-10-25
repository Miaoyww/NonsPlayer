import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/playback/web-audio.ts', import.meta.url), 'utf8'),
    {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }
  ).outputText,
  { exports, setTimeout, clearTimeout, require: () => ({ releaseBrowserOutput() {} }) }
)
const settle = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve()
}

class AudioStub extends EventTarget {
  paused = true
  currentTime = 0
  duration = 180
  readyState = 0
  seeking = false
  volume = 1
  load() {}
  removeAttribute() {
    this.src = ''
  }
  pause() {
    this.paused = true
  }
  play() {
    this.paused = false
    return Promise.resolve()
  }
  event(name) {
    this.dispatchEvent(new Event(name))
  }
}
const command = (serial, patch = {}) => ({
  serial,
  source: 'http://127.0.0.1:1234/token',
  seekId: 1,
  positionMs: 42000,
  startMs: 0,
  endMs: null,
  playing: true,
  volume: 0.5,
  fades: false,
  crossfadeMs: 0,
  deviceId: null,
  next: null,
  ...patch
})
function harness(send) {
  const reports = [],
    elements = [],
    errors = []
  const gains = []
  const output = {
    volumes: [],
    latencySeconds: 0,
    setVolume(value) {
      this.volumes.push(value)
    },
    attach() {
      const gain = {
        ramps: [],
        ramp(value, duration) {
          this.ramps.push([value, duration])
        },
        release() {}
      }
      gains.push(gain)
      return gain
    },
    async resume() {},
    async suspend() {},
    async setDevice() {},
    latency() {
      return this.latencySeconds
    }
  }
  const executor = exports.createWebAudio(
    send ?? (async (value) => reports.push(value)),
    (error) => errors.push(error),
    () => {
      const element = new AudioStub()
      elements.push(element)
      return element
    },
    () => output
  )
  return { executor, reports, elements, errors, gains, output }
}

test('restores position before playback and retains it across volume and pause/resume intents', async () => {
  const h = harness()
  h.executor.apply(command(1))
  const audio = h.elements[0]
  assert.equal(audio.paused, true)
  audio.readyState = 4
  audio.event('loadedmetadata')
  await settle()
  assert.equal(audio.currentTime, 42)
  assert.equal(audio.paused, false)
  audio.currentTime = 50
  h.executor.apply(command(2, { volume: 0.2 }))
  assert.equal(audio.currentTime, 50)
  assert.equal(audio.volume, 1)
  assert.equal(h.output.volumes.at(-1), 0.2)
  h.executor.apply(command(3, { playing: false }))
  assert.equal(audio.paused, true)
  h.executor.apply(command(4))
  await settle()
  assert.equal(audio.currentTime, 50)
  assert.equal(audio.paused, false)
  h.executor.apply(command(5, { seekId: 2, positionMs: 60000 }))
  assert.equal(audio.currentTime, 60)
  await settle()
  assert.equal(h.errors.length, 0)
  h.executor.dispose()
})

test('replaced resources ignore old events and stale controls; stop acknowledges actual position after pause', async () => {
  const h = harness()
  h.executor.apply(command(1))
  const old = h.elements[0]
  h.executor.apply(command(3, { source: 'http://127.0.0.1:1234/other' }))
  h.executor.apply(command(2))
  assert.equal(h.elements.length, 2)
  old.event('error')
  old.event('ended')
  const current = h.elements[1]
  current.readyState = 4
  current.event('loadedmetadata')
  current.currentTime = 57
  h.executor.apply(command(4, { source: null, playing: false }))
  assert.equal(current.paused, true)
  await settle()
  assert.equal(h.reports.at(-1).kind, 'stopped')
  assert.equal(h.reports.at(-1).positionMs, 57000)
  assert.equal(
    h.reports.some((r) => r.kind === 'error' || r.kind === 'ended'),
    false
  )
  h.executor.dispose()
})

test('CUE offsets, browser media fragments and end feedback share the file timeline', async () => {
  const h = harness()
  h.executor.apply(command(1, { startMs: 100000, endMs: 160000, positionMs: 5000 }))
  const audio = h.elements[0]
  assert.match(audio.src, /#t=100,160$/)
  audio.readyState = 4
  audio.event('loadedmetadata')
  await settle()
  assert.equal(audio.currentTime, 105)
  audio.currentTime = 160
  audio.event('timeupdate')
  audio.event('ended')
  await settle()
  assert.equal(audio.paused, true)
  assert.equal(h.reports.filter((r) => r.kind === 'ended').length, 1)
  h.executor.dispose()
})

test('reports stay bounded under slow IPC and a terminal event survives later progress', async () => {
  let complete
  const calls = []
  const h = harness((value) => {
    calls.push(value)
    return new Promise((resolve) => {
      complete = resolve
    })
  })
  h.executor.apply(command(1))
  const audio = h.elements[0]
  audio.readyState = 4
  audio.paused = false
  for (let i = 0; i < 1000; i++) {
    audio.currentTime = i / 10
    audio.event('timeupdate')
  }
  audio.event('error')
  audio.event('timeupdate')
  assert.equal(calls.length, 1)
  complete()
  await settle()
  assert.equal(calls.length, 2)
  assert.equal(calls[1].kind, 'error')
  h.executor.dispose()
  complete()
})

test('late play rejection from an old source cannot fail the current source', async () => {
  const h = harness()
  h.executor.apply(command(1))
  const first = h.elements[0]
  let reject
  first.play = () =>
    new Promise((_, fail) => {
      reject = fail
    })
  first.readyState = 4
  first.event('loadedmetadata')
  await settle()
  h.executor.apply(command(2, { source: 'http://127.0.0.1:1234/other' }))
  reject(Error('aborted'))
  await settle()
  assert.equal(
    h.reports.some((r) => r.kind === 'error'),
    false
  )
  h.executor.dispose()
})

test('preloads one successor and promotes the same element after overlapping playback starts', async () => {
  const h = harness()
  const next = { source: 'http://127.0.0.1:1234/next', startMs: 0, endMs: null }
  h.executor.apply(command(1, { positionMs: 0, crossfadeMs: 3000, next }))
  assert.equal(h.elements.length, 2)
  const [old, incoming] = h.elements
  old.readyState = incoming.readyState = 4
  old.event('loadedmetadata')
  incoming.event('loadedmetadata')
  await settle()
  old.currentTime = 178
  old.event('timeupdate')
  await settle()
  assert.equal(old.paused, false)
  assert.equal(incoming.paused, false)
  const transition = h.reports.find((r) => r.kind === 'transition')
  assert.equal(transition.nextSource, next.source)
  assert.deepEqual(h.gains[0].ramps.at(-1), [0, 2])
  assert.deepEqual(h.gains[1].ramps.at(-1), [1, 2])
  incoming.currentTime = 0.5
  h.executor.apply(command(2, { source: next.source, positionMs: 0, seekId: 2, crossfadeMs: 3000 }))
  assert.equal(h.elements.length, 2)
  assert.equal(incoming.currentTime, 0.5)
  h.executor.apply(command(3, { source: next.source, seekId: 2, crossfadeMs: 3000, volume: 0 }))
  assert.equal(h.output.volumes.at(-1), 0)
  assert.deepEqual(h.gains[1].ramps.at(-1), [1, 2])
  h.executor.dispose()
  assert.equal(old.paused, true)
  assert.equal(incoming.paused, true)
})

test('preload failure retains the current song and requests normal end-of-track fallback', async () => {
  const h = harness()
  const next = { source: 'http://127.0.0.1:1234/unavailable', startMs: 0, endMs: null }
  h.executor.apply(command(1, { positionMs: 0, crossfadeMs: 3000, next }))
  const [current, incoming] = h.elements
  current.readyState = 4
  current.event('loadedmetadata')
  await settle()
  incoming.event('error')
  await settle()
  assert.equal(current.paused, false)
  assert.equal(h.reports.at(-1).kind, 'preloadError')
  assert.equal(
    h.reports.some((r) => r.kind === 'error'),
    false
  )
  current.event('ended')
  await settle()
  assert.equal(h.reports.at(-1).kind, 'ended')
  h.executor.dispose()
})

test('output latency adjusts playing progress while stop acknowledges the raw media timeline', async () => {
  const h = harness()
  h.output.latencySeconds = 0.15
  h.executor.apply(command(1))
  const audio = h.elements[0]
  audio.readyState = 4
  audio.event('loadedmetadata')
  await settle()
  audio.currentTime = 50
  audio.event('timeupdate')
  await settle()
  assert.equal(h.reports.at(-1).positionMs, 49850)
  h.executor.apply(command(2, { source: null, playing: false }))
  await settle()
  assert.equal(h.reports.at(-1).positionMs, 50000)
  h.executor.dispose()
})

test('rapid resume reverses a pending pause envelope without stopping the element', async () => {
  const h = harness()
  h.executor.apply(command(1, { fades: true }))
  const audio = h.elements[0]
  audio.readyState = 4
  audio.event('loadedmetadata')
  await settle()
  h.executor.apply(command(2, { playing: false, fades: true }))
  assert.equal(audio.paused, false)
  assert.deepEqual(h.gains[0].ramps.at(-1), [0, 0.2])
  h.executor.apply(command(3, { playing: true, fades: true }))
  assert.deepEqual(h.gains[0].ramps.at(-1), [1, 0.2])
  assert.equal(audio.paused, false)
  h.executor.dispose()
})

test('stop identifies the incoming song if an engine switch beats the promotion acknowledgement', async () => {
  const h = harness()
  const next = { source: 'http://127.0.0.1:1234/next', startMs: 0, endMs: null }
  h.executor.apply(command(1, { positionMs: 0, crossfadeMs: 3000, next }))
  const [old, incoming] = h.elements
  old.readyState = incoming.readyState = 4
  old.event('loadedmetadata')
  incoming.event('loadedmetadata')
  await settle()
  old.currentTime = 178
  old.event('timeupdate')
  await settle()
  incoming.currentTime = 0.25
  h.executor.apply(command(2, { source: null, playing: false }))
  await settle()
  const stopped = h.reports.at(-1)
  assert.equal(stopped.kind, 'stopped')
  assert.equal(stopped.nextSource, next.source)
  assert.equal(stopped.positionMs, 250)
  assert.equal(stopped.durationMs, 180000)
  h.executor.dispose()
})
