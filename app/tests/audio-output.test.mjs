import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import ts from 'typescript'

const exports = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(new URL('../src/features/playback/audio-output.ts', import.meta.url), 'utf8'),
    {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }
  ).outputText,
  { exports, HTMLMediaElement: class {} }
)

function context() {
  const nodes = [],
    calls = []
  const param = () => ({
    value: 1,
    cancelScheduledValues(time) {
      calls.push(['cancel', time])
    },
    setValueAtTime(value, time) {
      calls.push(['hold', value, time])
    },
    linearRampToValueAtTime(value, time) {
      calls.push(['ramp', value, time])
    },
    setTargetAtTime() {}
  })
  const node = () => {
    const n = {
      connections: [],
      connect(target) {
        this.connections.push(target)
      },
      disconnect() {}
    }
    nodes.push(n)
    return n
  }
  return {
    nodes,
    calls,
    currentTime: 0,
    baseLatency: 0.05,
    outputLatency: 0.1,
    destination: {},
    state: 'suspended',
    createGain() {
      return { ...node(), gain: param() }
    },
    createDynamicsCompressor() {
      return {
        ...node(),
        threshold: param(),
        knee: param(),
        ratio: param(),
        attack: param(),
        release: param()
      }
    },
    createMediaElementSource() {
      return node()
    },
    async resume() {
      this.state = 'running'
    },
    async suspend() {
      this.state = 'suspended'
    },
    async close() {
      this.state = 'closed'
    },
    async setSinkId() {}
  }
}

test('an interrupted fade starts its reverse from the current interpolated gain', () => {
  const ctx = context()
  const output = exports.createAudioOutput(() => ctx)
  const voice = output.attach({})
  voice.ramp(0, 0.2)
  ctx.currentTime = 0.05
  voice.ramp(1, 0.2)
  assert.deepEqual(ctx.calls.slice(-3), [
    ['cancel', 0.05],
    ['hold', 0.75, 0.05],
    ['ramp', 1, 0.25]
  ])
  assert.ok(Math.abs(output.latency() - 0.15) < 1e-9)
  output.dispose()
})

test('device changes serialize, and a rejected selection does not block a later default selection', async () => {
  const ctx = context()
  const devices = []
  let complete
  ctx.setSinkId = async (id) => {
    devices.push(id)
    if (id === 'first')
      await new Promise((resolve) => {
        complete = resolve
      })
    if (id === 'missing') throw Error('device removed')
  }
  const output = exports.createAudioOutput(() => ctx)
  const first = output.setDevice('first'),
    second = output.setDevice('second')
  await Promise.resolve()
  assert.deepEqual(devices, ['first'])
  complete()
  await Promise.all([first, second])
  await assert.rejects(output.setDevice('missing'), /device removed/)
  await output.setDevice(null)
  assert.deepEqual(devices, ['first', 'second', 'missing', ''])
  output.dispose()
})

test('resume waits for an in-flight context suspension and restores running output', async () => {
  const ctx = context()
  let complete
  ctx.suspend = async () => {
    await new Promise((resolve) => {
      complete = resolve
    })
    ctx.state = 'suspended'
  }
  const output = exports.createAudioOutput(() => ctx)
  await output.resume()
  const pause = output.suspend()
  await Promise.resolve()
  const resume = output.resume()
  complete()
  await Promise.all([pause, resume])
  assert.equal(ctx.state, 'running')
  output.dispose()
})
