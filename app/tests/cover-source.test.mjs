import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

function harness({ intersecting = true } = {}) {
  const slots = [],
    effects = [],
    requests = [],
    deleted = []
  let cursor = 0
  const modules = {
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
      useRef() {
        const i = cursor++
        return (slots[i] ??= { current: {} })
      },
      useEffect(fn, deps) {
        const i = cursor++,
          previous = slots[i]
        if (!previous || deps.some((dep, j) => dep !== previous.deps[j])) {
          previous?.cleanup?.()
          slots[i] = { deps }
          effects.push(() => {
            slots[i].cleanup = fn()
          })
        }
      }
    },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/player': {
      coverSource: (cover) => cover,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    },
    '@/lib/runtime-cache': {
      coverCache: { delete: (key) => deleted.push(key) },
      requestKey: (command, args) => `${command}:${JSON.stringify(args)}`
    },
    '@/features/local/use-local-options': { useLocalOptions: () => ({ showCovers: true }) }
  }
  const exports = {}
  const { outputText } = ts.transpileModule(
    readFileSync(new URL('../src/components/music/use-cover-source.ts', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS } }
  )
  runInNewContext(outputText, {
    exports,
    URL,
    require(name) {
      assert.ok(name in modules, name)
      return modules[name]
    }
  })
  const components = {}
  const jsx = (type, props) => ({ type, props })
  Object.assign(modules, {
    'react/jsx-runtime': { jsx, jsxs: jsx },
    'lucide-react': {},
    '@/components/ui/skeleton': { Skeleton: 'skeleton' },
    '@/components/music/use-cover-source': exports
  })
  const componentText = ts.transpileModule(
    readFileSync(new URL('../src/components/music/cover.tsx', import.meta.url), 'utf8'),
    { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
  ).outputText
  runInNewContext(componentText, {
    exports: components,
    require: (name) => modules[name],
    IntersectionObserver: class {
      constructor(callback) {
        this.callback = callback
      }
      observe() {
        this.callback([{ isIntersecting: intersecting }])
      }
      disconnect() {}
    }
  })
  return {
    requests,
    deleted,
    renderCover(cover, full = false, props = {}) {
      cursor = 0
      const tree = components.Cover({ cover, ...props })
      effects.splice(0).forEach((fn) => fn())
      return full ? tree : tree.props.children.find((child) => child?.type === 'img')
    },
    render(cover, enabled = true) {
      cursor = 0
      const value = exports.useCoverImageSource(cover, enabled)
      effects.splice(0).forEach((fn) => fn())
      return value
    }
  }
}
const cover = 'https://p1.music.126.net/album.jpg'
const settle = async () => {
  await Promise.resolve()
  await Promise.resolve()
}

test('eager fullscreen cover loads and becomes visible without an intersection notification', async () => {
  const h = harness({ intersecting: false })
  const props = { loading: 'eager' }
  h.renderCover(cover, true, props)
  h.renderCover(cover, true, props)
  assert.equal(h.requests.length, 1, 'fullscreen cover must request its image during slide-in')
  h.requests[0].resolve('data:image/jpeg;base64,ready')
  await settle()
  const image = h.renderCover(cover, false, props)
  assert.equal(image.props.loading, 'eager', 'do not defer the already visible player image again')
  image.props.onLoad()
  const tree = h.renderCover(cover, true, props)
  assert.equal(tree.props.children[0].props['data-ready'], true)
  assert.equal(tree.props.children[1].props['data-ready'], true)
})

test('list covers still wait for intersection before requesting images', () => {
  const h = harness({ intersecting: false })
  h.renderCover(cover)
  h.renderCover(cover)
  assert.equal(h.requests.length, 0)
})

test('the actual Cover component recovers from a cached image error', async () => {
  const h = harness()
  h.renderCover(cover)
  h.renderCover(cover)
  h.requests[0].resolve('data:image/jpeg;base64,broken')
  await settle()
  const image = h.renderCover(cover)
  assert.equal(image.type, 'img')
  image.props.onError()
  assert.equal(h.renderCover(cover).props.src, cover)
})

test('cached image load errors invalidate the broken entry and fall back to the original URL', async () => {
  const h = harness()
  h.render(cover)
  h.requests[0].resolve('data:image/jpeg;base64,broken')
  await settle()
  let image = h.render(cover)
  assert.match(image.source, /^data:/)
  image.onError()
  image = h.render(cover)
  assert.equal(image.source, cover)
  assert.equal(h.deleted.length, 1)
  assert.match(h.deleted[0], /512y512/)
  image.onError()
  assert.equal(h.render(cover).source, undefined)
  assert.equal(
    h.requests.filter((r) => r.command === 'runtime_cover').length,
    1,
    'failure recovery must not cause a request loop'
  )
  assert.equal(h.requests.filter((r) => r.command === 'runtime_cover_invalidate').length, 1)
  assert.match(h.requests.at(-1).args.url, /512y512/)
})

test('request errors fall back, reactivation retries, and late previous-cover responses are ignored', async () => {
  const h = harness()
  h.render(cover)
  h.requests[0].reject(Error('offline'))
  await settle()
  assert.equal(h.render(cover).source, cover)
  h.render(cover, false)
  h.render(cover)
  const next = `${cover}?new=1`
  h.render(next)
  h.requests[1].resolve('data:image/jpeg;base64,old')
  await settle()
  assert.equal(h.render(next).source, undefined)
  h.requests[2].resolve('data:image/jpeg;base64,new')
  await settle()
  assert.equal(h.render(next).source, 'data:image/jpeg;base64,new')
})

test('Cover keeps its skeleton until image load, shows it again for a new source, and clears it after final failure', async () => {
  const h = harness()
  h.renderCover(cover)
  assert.equal(h.renderCover(cover, true).props.children[0].type, 'skeleton')
  h.requests[0].resolve('data:image/jpeg;base64,ready')
  await settle()
  let tree = h.renderCover(cover, true)
  let image = tree.props.children[1]
  assert.equal(tree.props.children[0].type, 'skeleton')
  assert.match(image.props.className, /opacity-0/)
  image.props.onLoad()
  tree = h.renderCover(cover, true)
  assert.equal(tree.props.children[0].props['data-ready'], true)
  assert.match(tree.props.children[0].props.className, /opacity-0/)
  assert.match(tree.props.children[1].props.className, /opacity-100/)
  assert.equal(tree.props.children[1].props['data-ready'], true)
  const next = `${cover}?next=1`
  tree = h.renderCover(next, true)
  assert.equal(tree.props.children[0].type, 'skeleton')
  assert.equal(tree.props.children[0].props['data-ready'], false)
  h.requests[1].reject(Error('offline'))
  await settle()
  image = h.renderCover(next)
  image.props.onError()
  tree = h.renderCover(next, true)
  assert.equal(tree.props.children[0], false)
  assert.equal(h.requests.length, 2)
})
