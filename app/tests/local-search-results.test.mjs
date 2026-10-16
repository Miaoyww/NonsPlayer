import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness() {
  const slots = [],
    effects = [],
    requests = []
  let cursor = 0
  const react = {
    useState(initial) {
      const index = cursor++
      if (!(index in slots)) slots[index] = initial
      return [
        slots[index],
        (value) => {
          slots[index] = typeof value === 'function' ? value(slots[index]) : value
        }
      ]
    },
    useRef(initial) {
      return (slots[cursor++] ??= { current: initial })
    },
    useCallback(fn, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i])) slots[index] = { deps, fn }
      return slots[index].fn
    },
    useMemo(factory, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i]))
        slots[index] = { deps, value: factory() }
      return slots[index].value
    },
    useEffect(fn, deps) {
      const index = cursor++,
        old = slots[index]
      if (!old || deps.some((value, i) => value !== old.deps[i])) {
        old?.cleanup?.()
        slots[index] = { deps }
        effects.push(() => {
          slots[index].cleanup = fn()
        })
      }
    }
  }
  const jsx = (type, props) => ({ type, props })
  const modules = {
    react,
    'react/jsx-runtime': { jsx, jsxs: jsx },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/components/animate-ui/components/base/tabs': {
      Tabs: 'tabs',
      TabsList: 'list',
      TabsTab: 'tab',
      TabsPanel: 'panel'
    },
    '@/lib/player': {
      errorText: String,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    },
    './player': { errorText: String }
  }
  function load(path, suffix = '') {
    const exports = {}
    runInNewContext(
      ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8') + suffix, {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }).outputText,
      {
        exports,
        require: (name) =>
          name === '@/components/error-notice' ? errorNoticeModule : (modules[name] ?? {})
      }
    )
    return exports
  }
  modules['@/lib/use-paged-list'] = load('../src/lib/use-paged-list.ts')
  const { LocalSearchResults } = load(
    '../src/features/local/local-page.tsx',
    '\nexport { LocalSearchResults };'
  )
  function render() {
    cursor = 0
    const tree = LocalSearchResults({ keyword: 'MO bius', refresh: 0, onError() {}, onNotice() {} })
    while (effects.length) effects.shift()()
    return tree
  }
  return { render, requests }
}
const settle = async () => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
}
const empty = { items: [], more: false }

test('local search hides navigation only after all three successful empty responses', async () => {
  const app = harness()
  assert.equal(app.render().type, 'tabs', 'initial state is not a completed empty search')
  assert.deepEqual(
    app.requests.map((request) => request.args.kind),
    ['song', 'artist', 'album']
  )
  app.requests[0].resolve(empty)
  app.requests[1].resolve(empty)
  await settle()
  assert.equal(app.render().type, 'tabs', 'a pending category keeps navigation visible')
  app.requests[2].resolve(empty)
  await settle()
  const tree = app.render()
  assert.equal(tree.type, 'div')
  assert.equal(tree.props.role, 'status')
  assert.match(tree.props.children[1].props.children.join(''), /MO bius/)
  assert.equal(app.requests.length, 3)
})

test('partial results keep the shared vertical tabs and switching does not query again', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve(empty)
  app.requests[1].resolve({ items: [{ id: 'artist', name: 'MO bius' }], more: false })
  app.requests[2].resolve(empty)
  await settle()
  const tree = app.render()
  assert.equal(tree.type, 'tabs')
  assert.equal(tree.props.orientation, 'vertical')
  assert.equal(tree.props.children[0].type, 'list')
  tree.props.onValueChange('artist')
  assert.equal(app.render().props.value, 'artist')
  assert.equal(app.requests.length, 3)
})

test('query errors remain visible through Results and retry can reach the global empty state', async () => {
  const app = harness()
  app.render()
  app.requests[0].resolve(empty)
  app.requests[1].reject(new Error('read failed'))
  app.requests[2].resolve(empty)
  await settle()
  const tree = app.render()
  assert.equal(tree.type, 'tabs')
  const artistResult = tree.props.children[1].props.children[0].props.children[1]
  assert.match(artistResult.props.list.error, /read failed/)
  const retry = artistResult.props.list.loadMore()
  assert.equal(app.requests[3].args.offset, 0)
  app.requests[3].resolve(empty)
  await retry
  assert.equal(app.render().props.className, 'search-empty')
})
