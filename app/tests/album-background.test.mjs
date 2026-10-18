import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import { coverImage } from './helpers/cover-image.mjs'

const cover = {
  small: 'https://y.gtimg.cn/small.jpg',
  original: 'https://y.gtimg.cn/original.jpg'
}
const data = 'data:image/jpeg;base64,cover'
function deferred() {
  let resolve, reject
  const promise = new Promise((yes, no) => {
    resolve = yes
    reject = no
  })
  return { promise, resolve, reject }
}

async function harness(t, { delayedImport = false } = {}) {
  const dom = new JSDOM('<div id="root"></div>', { pretendToBeVisual: true })
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    IS_REACT_ACT_ENVIRONMENT: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  Object.assign(globalThis, {
    window: dom.window,
    document: dom.window.document,
    IS_REACT_ACT_ENVIRONMENT: true
  })
  const requests = [],
    invalidated = [],
    deleted = [],
    renderers = [],
    errors = []
  const importGate = deferred()
  let hidden = false
  Object.defineProperty(document, 'visibilityState', { get: () => (hidden ? 'hidden' : 'visible') })
  const amll = {
    MeshGradientRenderer: class {},
    BackgroundRender: {
      new() {
        const canvas = document.createElement('canvas')
        const loading = deferred()
        const renderer = {
          albums: [],
          disposed: false,
          paused: true,
          loading,
          pause() {
            this.paused = true
          },
          resume() {
            this.paused = false
          },
          setFPS() {},
          setRenderScale() {},
          setFlowSpeed() {},
          setHasLyric() {},
          setStaticMode() {},
          getElement: () => canvas,
          setAlbum(image) {
            this.albums.push(image)
            return loading.promise
          },
          dispose() {
            this.disposed = true
            canvas.remove()
          }
        }
        renderers.push(renderer)
        return renderer
      }
    }
  }
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    'motion/react': { useReducedMotion: () => false },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/diagnostics': { reportCaughtError: (...args) => errors.push(args) },
    '@/lib/after-first-paint': {
      afterFirstPaint(work) {
        work()
        return () => {}
      }
    },
    '@/features/music/cover-image': coverImage,
    '@/features/local/use-local-options': { useLocalOptions: () => ({ showCovers: true }) },
    '@/features/lyrics/use-lyrics-settings': { useLyricsSettings: () => ({ backgroundSpeed: 1 }) },
    '@/lib/runtime-cache': {
      coverCache: { peekFresh: () => undefined, delete: (key) => deleted.push(key) },
      requestKey: (command, args) => command + ':' + args.url
    },
    '@/lib/player': {
      coverSource: (url) => url,
      nativeCall(command, args) {
        if (command === 'runtime_cover_invalidate') {
          invalidated.push(args.url)
          return Promise.resolve()
        }
        assert.equal(command, 'runtime_cover')
        const result = deferred()
        requests.push({ ...result, url: args.url })
        return result.promise
      }
    },
    '@applemusic-like-lyrics/core': delayedImport ? importGate.promise : amll
  }
  function load(path) {
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        document,
        require(name) {
          assert.ok(name in modules, name)
          return modules[name]
        }
      }
    )
    return exports
  }
  modules['@/components/music/use-cover-source'] = load(
    '../src/components/music/use-cover-source.ts'
  )
  const { AlbumBackground } = load('../src/features/lyrics/album-background.tsx')
  const root = createRoot(document.getElementById('root'))
  let props = { cover, playing: true, hasLyrics: true }
  const render = (next = {}) =>
    act(async () => {
      props = { ...props, ...next }
      root.render(React.createElement(AlbumBackground, props))
    })
  t.after(async () => {
    await act(async () => root.unmount())
    Object.assign(globalThis, previous)
    dom.window.close()
  })
  await render()
  return {
    render,
    requests,
    invalidated,
    deleted,
    renderers,
    errors,
    get image() {
      return document.querySelector('img')
    },
    get ready() {
      return document.querySelector('[data-ready]').dataset.ready === 'true'
    },
    event: (image, name) => act(async () => image.dispatchEvent(new dom.window.Event(name))),
    finish: (work) =>
      act(async () => {
        work()
      }),
    resolveImport: () => act(async () => importGate.resolve(amll)),
    visibility: (value) =>
      act(async () => {
        hidden = !value
        document.dispatchEvent(new dom.window.Event('visibilitychange'))
      })
  }
}

test('background waits for a successful image load and passes that image to AMLL', async (t) => {
  const h = await harness(t)
  assert.equal(h.renderers.length, 0)
  await h.finish(() => h.requests[0].resolve(data))
  const image = h.image
  assert.equal(image.src, data)
  assert.equal(h.renderers.length, 0, 'a host response alone does not prove the image is usable')
  await h.event(image, 'load')
  assert.equal(h.renderers.length, 1)
  assert.equal(
    h.renderers[0].albums[0],
    image,
    'AMLL must reuse the loaded image instead of fetching a string'
  )
  assert.equal(h.ready, false)
  await h.finish(() => h.renderers[0].loading.resolve())
  assert.equal(h.ready, true)
  assert.equal(h.renderers[0].paused, false)
})

test('decode failure invalidates both caches, tries the next host size and stops at a placeholder', async (t) => {
  const h = await harness(t)
  await h.finish(() => h.requests[0].resolve(data))
  await h.event(h.image, 'error')
  assert.deepEqual(h.invalidated, [cover.small])
  assert.equal(h.deleted.length, 1)
  assert.equal(h.requests[1].url, cover.original)
  assert.equal(h.renderers.length, 0)
  await h.finish(() => h.requests[1].reject(Error('HTTP 404')))
  assert.equal(h.image, null)
  assert.equal(h.ready, false)
  assert.equal(h.renderers.length, 0)
  await h.render()
  assert.equal(h.requests.length, 2)
  await h.render({ active: false })
  await h.render({ active: true })
  assert.equal(h.requests.length, 3, 'reopening retries the first host candidate')
})

test('switching songs rejects stale image events and renderer completion', async (t) => {
  const h = await harness(t)
  await h.finish(() => h.requests[0].resolve(data))
  const oldImage = h.image
  await h.event(oldImage, 'load')
  const oldRenderer = h.renderers[0]
  await h.render({ cover: 'https://other.example/next.jpg' })
  assert.equal(oldRenderer.disposed, true)
  await h.finish(() => h.requests[1].resolve(data + '-next'))
  const image = h.image
  assert.notEqual(image, oldImage)
  await h.event(oldImage, 'load')
  await h.event(oldImage, 'error')
  await h.finish(() => oldRenderer.loading.resolve())
  assert.equal(h.ready, false)
  assert.equal(h.renderers.length, 1)
  assert.equal(h.invalidated.length, 0)
  await h.event(image, 'load')
  assert.equal(h.renderers[1].albums[0], image)
  await h.finish(() => h.renderers[1].loading.resolve())
  assert.equal(h.ready, true)
})

test('hidden and inactive backgrounds stop rendering, and a new activation reloads safely', async (t) => {
  const h = await harness(t)
  await h.finish(() => h.requests[0].resolve(data))
  await h.event(h.image, 'load')
  const renderer = h.renderers[0]
  await h.finish(() => renderer.loading.resolve())
  await h.visibility(false)
  assert.equal(renderer.disposed, true)
  assert.equal(h.image, null)
  await h.visibility(true)
  assert.equal(h.requests.length, 2)
  await h.render({ active: false })
  await h.finish(() => h.requests[1].resolve(data + '-stale'))
  assert.equal(h.image, null)
  assert.equal(h.renderers.length, 1)
  await h.render({ active: true })
  await h.finish(() => h.requests[2].resolve(data))
  assert.equal(h.renderers.length, 1, 'a fresh image must load even if it reuses the same URI')
  await h.event(h.image, 'load')
  assert.equal(h.renderers.length, 2)
})

test('closing during AMLL import prevents late renderer creation', async (t) => {
  const h = await harness(t, { delayedImport: true })
  await h.finish(() => h.requests[0].resolve(data))
  await h.event(h.image, 'load')
  await h.render({ active: false })
  await h.resolveImport()
  assert.equal(h.renderers.length, 0)
  assert.equal(h.ready, false)
})

test('renderer failure retains the loaded fallback without invalidating a valid image', async (t) => {
  const h = await harness(t)
  await h.finish(() => h.requests[0].resolve(data))
  await h.event(h.image, 'load')
  await h.finish(() => h.renderers[0].loading.reject(Error('WebGL unavailable')))
  assert.equal(h.ready, false)
  assert.equal(h.renderers[0].disposed, true)
  assert.equal(h.image.src, data)
  assert.equal(h.invalidated.length, 0)
  assert.equal(h.requests.length, 1)
})
