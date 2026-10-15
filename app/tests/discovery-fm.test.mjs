import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness() {
  const slots = [],
    effects = [],
    requests = [],
    errors = []
  let cursor = 0,
    profile = { userId: 1 },
    player = { index: null, queue: [], status: 'stopped' }
  const jsx = (type, props) => ({ type, props })
  const react = {
    useState(initial) {
      const i = cursor++
      if (!(i in slots)) slots[i] = initial
      return [
        slots[i],
        (value) => {
          slots[i] = typeof value === 'function' ? value(slots[i]) : value
        }
      ]
    },
    useRef(current) {
      return (slots[cursor++] ??= { current })
    },
    useEffect(fn, deps) {
      const i = cursor++,
        old = slots[i]
      if (!old || deps.some((dep, j) => dep !== old.deps[j])) {
        old?.cleanup?.()
        slots[i] = { deps }
        effects.push(() => {
          slots[i].cleanup = fn()
        })
      }
    }
  }
  const modules = {
    '@/features/discovery/discovery-api': {
      dislikeTrack: (track) =>
        modules['@/lib/player'].nativeCall('music_write', {
          source: 'netease',
          request: {
            operation: 'dislike',
            track: { source: 'netease', kind: 'track', id: String(track.source.id) }
          }
        }),
      discoveryTracks: () =>
        modules['@/lib/player'].nativeCall('music_query', {
          request: { operation: 'recommendedTracks', kind: 'fm' }
        })
    },
    react,
    'react/jsx-runtime': { jsx, jsxs: jsx },
    'lucide-react': {},
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/features/account/account': { useAccount: () => ({ profile }) },
    '@/features/workspace/music-navigation': { useMusicNavigation: () => ({ navigate() {} }) },
    '@/lib/player': {
      usePlayer: () => player,
      getPlayer: () => player,
      errorText: String,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    }
  }
  const source =
    readFileSync(new URL('../src/features/discovery/discovery.tsx', import.meta.url), 'utf8') +
    '\nexport { PrivateFM };'
  const exports = {}
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  runInNewContext(outputText, { exports, require: (name) => modules[name] ?? {} })
  function render() {
    cursor = 0
    const tree = exports.PrivateFM({ onError: (error) => errors.push(error) })
    while (effects.length) effects.shift()()
    return tree
  }
  function find(node, label) {
    if (!node || typeof node !== 'object') return
    if (node.props?.['aria-label'] === label) return node.props
    for (const child of [node.props?.children].flat()) {
      const match = find(child, label)
      if (match) return match
    }
  }
  return {
    requests,
    errors,
    render,
    button: (label) => find(render(), label),
    setPlayer: (value) => {
      player = value
    },
    changeAccount: () => {
      profile = { userId: 2 }
      render()
    }
  }
}
const track = (id) => ({
  key: `netease:${id}`,
  title: `Song ${id}`,
  artist: 'Artist',
  album: 'Album',
  source: { kind: 'netease', id }
})
const settle = async () => {
  for (let i = 0; i < 6; i++) await Promise.resolve()
}

test('FM starts once on repeated clicks and skips through the existing queue', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve([track(1), track(2), track(3)])
  await settle()
  const play = app.button('播放私人 FM')
  play.onClick()
  play.onClick()
  assert.equal(app.requests.length, 2)
  assert.equal(app.requests[1].command, 'play_private_fm')
  assert.deepEqual(Array.from(app.requests[1].args.keys), ['netease:1', 'netease:2', 'netease:3'])
  app.requests[1].resolve()
  await settle()
  app.setPlayer({
    privateFmSession: 1,
    index: 0,
    queue: [track(1), track(2), track(3)],
    status: 'playing'
  })
  app.button('下一首私人 FM').onClick()
  await settle()
  assert.equal(app.requests.length, 3)
  assert.equal(app.requests[2].command, 'player_action')
  assert.equal(app.requests[2].args.action, 'next')
  app.requests[2].resolve()
  await settle()
})

test('failed dislike preserves the current FM batch and exposes a retry', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve([track(1), track(2)])
  await settle()
  app.button('不喜欢这首歌').onClick()
  assert.equal(app.requests[1].command, 'music_write')
  app.requests[1].reject(new Error('denied'))
  await settle()
  assert.equal(app.requests.length, 2)
  assert.equal(app.button('播放私人 FM').disabled, false)
  assert.match(JSON.stringify(app.render()), /denied/)
})

test('account change discards an old FM batch', async () => {
  const app = harness()
  app.render()
  app.changeAccount()
  app.requests[0].resolve([track(1)])
  await settle()
  assert.equal(app.button('播放私人 FM').disabled, true)
  app.requests[1].resolve([track(2)])
  await settle()
  app.button('播放私人 FM').onClick()
  assert.deepEqual(Array.from(app.requests[2].args.keys), ['netease:2'])
  app.requests[2].resolve()
  await settle()
})

test('editing the queue while dislike is pending does not skip the replacement queue', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve([track(1), track(2), track(3)])
  await settle()
  app.setPlayer({
    privateFmSession: 7,
    index: 1,
    queue: [track(1), track(2), track(3)],
    status: 'playing'
  })
  app.button('不喜欢这首歌').onClick()
  app.setPlayer({ privateFmSession: null, index: 0, queue: [track(9)], status: 'playing' })
  app.requests[1].resolve()
  await settle()
  assert.equal(app.requests.length, 2)
})
