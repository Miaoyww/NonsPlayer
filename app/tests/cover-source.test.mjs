import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness({ intersecting = true, tauri = true, cached = false } = {}) {
  const slots = [],
    effects = [],
    requests = [],
    deleted = []
  let cursor = 0
  const modules = {
    '@/lib/after-first-paint': {
      afterFirstPaint(work) {
        work()
        return () => {}
      }
    },
    '@/features/music/cover-image': coverImage,
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
    '@tauri-apps/api/core': { isTauri: () => tauri },
    '@/lib/player': {
      coverSource: (cover) => cover,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    },
    '@/lib/runtime-cache': {
      coverCache: {
        peekFresh: () =>
          cached ? { access: 'valid', data: 'data:image/jpeg;base64,ready' } : undefined,
        delete: (key) => deleted.push(key)
      },
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
    render(cover, enabled = true, size = 'normal') {
      cursor = 0
      const value = exports.useCoverImageSource(cover, enabled, size)
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

test('the actual Cover component shows a placeholder after its only cached image fails', async () => {
  const h = harness()
  h.renderCover(cover)
  h.renderCover(cover)
  h.requests[0].resolve('data:image/jpeg;base64,broken')
  await settle()
  const image = h.renderCover(cover)
  assert.equal(image.type, 'img')
  image.props.onError()
  assert.equal(h.renderCover(cover), undefined)
})

test('cached image load errors invalidate both caches and never expose a remote URL', async () => {
  const h = harness()
  h.render(cover)
  h.requests[0].resolve('data:image/jpeg;base64,broken')
  await settle()
  let image = h.render(cover)
  assert.match(image.source, /^data:/)
  image.onError()
  image = h.render(cover)
  assert.equal(image.source, undefined)
  assert.equal(image.pending, false)
  assert.equal(h.deleted.length, 1)
  assert.ok(h.deleted[0].includes(cover))
  image.onError()
  assert.equal(h.render(cover).source, undefined)
  assert.equal(
    h.requests.filter((r) => r.command === 'runtime_cover').length,
    1,
    'failure recovery must not cause a request loop'
  )
  assert.equal(h.requests.filter((r) => r.command === 'runtime_cover_invalidate').length, 1)
  assert.equal(h.requests.at(-1).args.url, cover)
})

test('request errors stop, reactivation retries, and late previous-cover responses are ignored', async () => {
  const h = harness()
  h.render(cover)
  h.requests[0].reject(Error('offline'))
  await settle()
  assert.equal(h.render(cover).source, undefined)
  assert.equal(h.render(cover).pending, false)
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
  assert.equal(image, undefined)
  tree = h.renderCover(next, true)
  assert.equal(tree.props.children[0], false)
  assert.equal(h.requests.length, 2)
})

const variants = Object.fromEntries(
  ['small', 'normal', 'large', 'exlarge', 'original'].map((size) => [
    size,
    `https://other.example/${size}.jpg`
  ])
)
test('semantic sizes select adapter URLs without adding platform parameters, and only full cover requests original', () => {
  for (const size of ['small', 'normal', 'large', 'exlarge', 'original']) {
    const h = harness()
    h.render(variants, true, size)
    assert.equal(h.requests[0].args.url, variants[size])
    assert.equal(h.requests.length, 1)
  }
  const h = harness({ intersecting: false })
  h.renderCover(variants, true, { loading: 'eager', size: 'original' })
  assert.equal(h.requests[0].args.url, variants.original)
})
test('a card degrades to small then original, invalidates decoded errors and never loops', async () => {
  const h = harness()
  h.render(variants)
  h.requests[0].reject(Error('normal unavailable'))
  await settle()
  h.render(variants)
  assert.equal(h.requests[1].args.url, variants.small)
  h.requests[1].resolve('data:image/jpeg;base64,broken')
  await settle()
  h.render(variants).onError()
  h.render(variants)
  const original = h.requests.find(
    (r) => r.command === 'runtime_cover' && r.args.url === variants.original
  )
  assert.ok(original)
  original.resolve('data:image/jpeg;base64,also-broken')
  await settle()
  h.render(variants).onError()
  assert.equal(h.render(variants).source, undefined)
  assert.equal(h.render(variants).pending, false)
  h.render(variants).onError()
  assert.equal(h.requests.filter((r) => r.command === 'runtime_cover').length, 3)
  assert.equal(h.deleted.length, 2)
})
test('size changes ignore older in-flight images and original failures can recover through exlarge', async () => {
  const h = harness()
  h.render(variants, true, 'small')
  h.render(variants, true, 'original')
  h.requests[0].resolve('data:image/jpeg;base64,old-small')
  await settle()
  assert.equal(h.render(variants, true, 'original').source, undefined)
  h.requests[1].reject(Error('original too big'))
  await settle()
  h.render(variants, true, 'original')
  assert.equal(h.requests[2].args.url, variants.exlarge)
  h.requests[2].resolve('data:image/jpeg;base64,media')
  await settle()
  assert.equal(h.render(variants, true, 'original').source, 'data:image/jpeg;base64,media')
})

test('desktop QQ covers exhaust host candidates without falling back through browser CORS', async () => {
  const h = harness()
  const original = 'https://y.gtimg.cn/music/photo_new/T002R800x800M0000026VO7R304gmO.jpg'
  const qq = { ...variants, small: original.replace('800x800', '150x150'), original }
  h.render(qq, true, 'small')
  h.requests[0].reject(Error('HTTP 404'))
  await settle()
  assert.equal(h.render(qq, true, 'small').source, undefined)
  assert.equal(h.requests[1].args.url, original)
  h.requests[1].reject(Error('source unavailable'))
  await settle()
  for (let i = 0; i < 5; i++) {
    const result = h.render(qq, true, 'small')
    assert.equal(result.source, undefined, 'AMLL must never receive a cross-origin fallback')
    assert.equal(result.pending, false)
    result.onError()
  }
  assert.equal(h.requests.length, 2, 'each unique host candidate is attempted only once')
})

test('host authorization rejection does not bypass the host even for display-only covers', async () => {
  const h = harness()
  h.renderCover(cover, true, { loading: 'eager' })
  h.requests[0].reject(Error('permissionDenied'))
  await settle()
  const tree = h.renderCover(cover, true, { loading: 'eager' })
  assert.equal(
    tree.props.children.some((child) => child?.type === 'img'),
    false
  )
  assert.equal(h.requests.length, 1)
})

test('non-Tauri previews retain display-only URLs without claiming a native cache hit', () => {
  const h = harness({ tauri: false })
  assert.equal(h.render(cover).source, cover)
  assert.equal(h.requests.length, 0)
  h.render(cover).onError()
  assert.equal(h.render(cover).source, undefined)
})

test('cached cover skips skeleton and reveal while retaining live host validation', async () => {
  const h = harness({ cached: true })
  const props = { loading: 'eager' }
  let tree = h.renderCover(cover, true, props)
  assert.equal(
    tree.props.children.some((child) => child?.type === 'skeleton'),
    false
  )
  assert.equal(h.requests.length, 1, 'cache hints must not bypass host authorization')
  assert.equal(
    tree.props.children.some((child) => child?.type === 'img'),
    false
  )
  h.requests[0].resolve('data:image/jpeg;base64,ready')
  await settle()
  tree = h.renderCover(cover, true, props)
  const image = tree.props.children.find((child) => child?.type === 'img')
  assert.equal(image.props['data-ready'], true, 'cached content needs no second load event')
  assert.equal(image.props['data-animate'], false)
  assert.equal(
    tree.props.children.some((child) => child?.type === 'skeleton'),
    false
  )
  image.props.onError()
  assert.equal(h.renderCover(cover, false, props), undefined)
  assert.equal(h.deleted.length, 1)
})

test('already complete local images skip skeleton and reveal on attachment', () => {
  const h = harness({ tauri: false })
  const props = { loading: 'eager' }
  const image = h.renderCover('asset://cover.jpg', false, props)
  image.props.ref({ complete: true, naturalWidth: 100 })
  const tree = h.renderCover('asset://cover.jpg', true, props)
  assert.equal(
    tree.props.children.some((child) => child?.type === 'skeleton'),
    false
  )
  assert.equal(
    tree.props.children.find((child) => child?.type === 'img').props['data-animate'],
    false
  )
})

test('missing local assets recover once and retry the repaired path without reusing the failed response', async () => {
  const h = harness()
  const local = 'C:\\Nons\\covers\\missing.img'
  let image = h.render(local)
  image.onError()
  image.onError()
  image = h.render(local)
  assert.equal(image.source, undefined)
  assert.equal(image.pending, true)
  assert.equal(h.requests.length, 1)
  assert.equal(h.requests[0].command, 'local_cover_repair')
  assert.equal(h.requests[0].args.url, local)
  h.requests[0].resolve(local)
  await settle()
  image = h.render(local)
  assert.ok(image.source.startsWith(`${local}?local-cover-repair=`))
  assert.equal(image.pending, false)
  image.onError()
  image = h.render(local)
  assert.equal(image.source, undefined)
  assert.equal(image.pending, false)
  image.onError()
  assert.equal(h.requests.length, 1, 'a failed repaired image must stop rather than loop')
  assert.equal(h.deleted.length, 0, 'local assets do not use the remote cover cache')
})

test('local repair failures stop and late repairs cannot replace another cover or a hidden image', async () => {
  const h = harness()
  const first = 'C:\\Nons\\covers\\first.img'
  const second = 'C:\\Nons\\covers\\second.img'
  h.render(first).onError()
  h.render(second)
  h.requests[0].resolve('C:\\Nons\\covers\\repaired-first.img')
  await settle()
  assert.equal(h.render(second).source, second)
  h.render(second).onError()
  h.render(second, false)
  h.requests[1].resolve('C:\\Nons\\covers\\repaired-second.img')
  await settle()
  assert.equal(h.render(second, false).source, undefined)
  h.render(second)
  h.render(second).onError()
  h.requests[2].reject(Error('source unavailable'))
  await settle()
  assert.equal(h.render(second).source, undefined)
  assert.equal(h.render(second).pending, false)
  assert.equal(h.requests.length, 3)
})
