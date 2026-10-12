import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness() {
  const slots = [],
    effects = [],
    requests = []
  let cursor = 0
  const sources = { amll: true, qq: true }
  const modules = {
    '@/components/music/loading': { TextSkeleton: 'div' },
    '@/features/local/use-local-preferences': {
      useLocalPreferences: () => ({ options: { artistSeparators: [] } })
    },
    react: {
      useState(initial) {
        const i = cursor++
        if (!(i in slots)) slots[i] = initial
        return [
          slots[i],
          (value) => {
            slots[i] = value
          }
        ]
      },
      useEffect(fn, deps) {
        const i = cursor++,
          old = slots[i]
        if (!old || deps.some((v, j) => v !== old.deps[j])) {
          old?.cleanup?.()
          slots[i] = { deps }
          effects.push(() => {
            slots[i].cleanup = fn()
          })
        }
      }
    },
    'react/jsx-runtime': {
      jsx: (type, props) => ({ type, props }),
      jsxs: (type, props) => ({ type, props })
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/features/lyrics/use-lyric-sources': {
      useLyricSources: () => ({ sources }),
      getLyricSources: () => sources
    },
    'lucide-react': { Heart: 'heart' },
    '@/features/lyrics/parse-lyrics': {
      parseLyrics: (value) => {
        if (value.broken) throw Error('解析失败')
        return value.content
      }
    },
    '@/lib/player': {
      errorText: (e) => e.message,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    }
  }
  function load(path) {
    const exports = {}
    const code = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText
    runInNewContext(code, { exports, require: (name) => modules[name] })
    return exports
  }
  modules['@/lib/player'] = modules['@/lib/player']
  modules['@/features/lyrics/load-lyrics'] = load('../src/features/lyrics/load-lyrics.ts')
  const { LyricExcerpt } = load('../src/features/lyrics/lyric-excerpt.tsx')
  return {
    requests,
    render(tracks, enabled = true) {
      cursor = 0
      const tree = LyricExcerpt({ tracks, enabled })
      effects.splice(0).forEach((fn) => fn())
      return tree
    },
    dispose() {
      slots.forEach((slot) => slot?.cleanup?.())
    }
  }
}
const tracks = [
  { key: 'one', title: '第一首', durationMs: 1000 },
  { key: 'two', title: '第二首', durationMs: 1000 },
  { key: 'three', title: '第三首' }
]
const lyrics = (text) => ({ source: 'netease', content: [{ words: [{ word: text }] }] })
const settle = async () => {
  for (let i = 0; i < 15; i++) await Promise.resolve()
}
const text = (tree) => JSON.stringify(tree.props.children)

test('first usable lyrics stop the chain; missing or credits-only first lyrics try the second song', async () => {
  for (const first of [lyrics('第一首歌词'), null, lyrics('纯音乐，请欣赏')]) {
    const h = harness()
    h.render(tracks)
    await settle()
    assert.equal(h.requests[0].args.key, 'one')
    h.requests[0].resolve(first)
    await settle()
    if (first?.content[0].words[0].word === '第一首歌词') {
      assert.equal(h.requests.length, 1)
      assert.match(text(h.render(tracks)), /第一首歌词/)
    } else {
      assert.equal(h.requests[1].args.key, 'two')
      h.requests[1].resolve(lyrics('第二首歌词'))
      await settle()
      assert.equal(h.requests.length, 2)
      assert.match(text(h.render(tracks)), /第二首歌词/)
    }
    h.dispose()
  }
})

test('two lyric failures use Hitokoto, and total failure displays actual reasons', async () => {
  for (const outcome of ['success', 'error', 'empty']) {
    const h = harness()
    h.render(tracks)
    await settle()
    h.requests[0].reject(Error('歌词请求超时'))
    await settle()
    h.requests[1].resolve(null)
    await settle()
    assert.equal(h.requests[2].command, 'discovery_hitokoto')
    if (outcome === 'error') h.requests[2].reject(Error('一言网络不可用'))
    else h.requests[2].resolve(outcome === 'empty' ? ' ' : '音乐相伴')
    await settle()
    const tree = h.render(tracks)
    if (outcome === 'success') assert.match(text(tree), /音乐相伴/)
    else {
      assert.match(text(tree), /歌词请求超时/)
      assert.match(text(tree), /第二首.*没有可用歌词/)
      assert.match(text(tree), outcome === 'error' ? /一言网络不可用/ : /返回内容为空/)
    }
    assert.equal(h.requests.length, 3, 'must not try a third song or loop after failure')
    h.dispose()
  }
})

test('disabled cards do not load; empty libraries use quotes; changing songs cancels the old fallback chain', async () => {
  const h = harness()
  h.render(tracks, false)
  await settle()
  assert.equal(h.requests.length, 0)
  h.render([], true)
  await settle()
  assert.equal(h.requests[0].command, 'discovery_hitokoto')
  h.render(tracks)
  await settle()
  h.requests[0].resolve('过期一言')
  await settle()
  assert.doesNotMatch(text(h.render(tracks)) ?? '', /过期一言/)
  h.dispose()
  h.requests[1].resolve(null)
  await settle()
  assert.equal(h.requests.length, 2, 'unmounted excerpts must not request the next song')
})
