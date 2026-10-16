import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function harness(saved = '[]', storageUnavailable = false, pageMode = false) {
  const storage = new Map([['nons-search-history', saved]])
  const slots = [],
    effects = [],
    requests = [],
    navigations = [],
    timers = new Map()
  let cursor = 0,
    timerId = 0,
    page = { view: 'library', query: '' }
  const jsx = (type, props) => ({ type, props })
  const react = {
    useState(initial) {
      const i = cursor++
      if (!(i in slots)) slots[i] = typeof initial === 'function' ? initial() : initial
      return [
        slots[i],
        (value) => {
          slots[i] = value
        }
      ]
    },
    useRef() {
      return (slots[cursor++] ??= { current: { focus() {}, blur() {} } })
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
    '@/features/music/use-music-sources': {
      useMusicSources: () => ({ preferences: { current: 'netease' }, revision: 0 })
    },
    '@/features/music/client': {
      musicClient: {
        query: (source, request) =>
          new Promise((resolve, reject) =>
            requests.push({
              command: 'music_query',
              args: { source, keyword: request.keyword },
              resolve: (items) => resolve({ type: 'suggestions', data: items }),
              reject
            })
          )
      }
    },
    react,
    'react/jsx-runtime': { jsx, jsxs: jsx },
    'lucide-react': { Search: 'icon', History: 'history-icon' },
    '@tauri-apps/api/core': { isTauri: () => true },
    '@base-ui/react/combobox': { Combobox: { Root: 'combo', Input: 'input' } },
    '@/components/ui/button': { Button: 'button' },
    '@/components/ui/input': { Input: 'input' },
    '@/components/ui/combobox': {
      ComboboxContent: 'popup',
      ComboboxItem: 'item',
      ComboboxList: 'list'
    },
    '@/features/workspace/music-navigation': {
      useMusicNavigation: () => ({ page, navigate: (...args) => navigations.push(args) })
    },
    '@/lib/player': {
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }))
    }
  }
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL('../src/features/search/music-search.tsx', import.meta.url), 'utf8'),
      {
        compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
      }
    ).outputText,
    {
      exports,
      require: (name) => modules[name],
      localStorage: {
        getItem(key) {
          if (storageUnavailable) throw Error('unavailable')
          return storage.get(key)
        },
        setItem(key, value) {
          if (storageUnavailable) throw Error('unavailable')
          storage.set(key, value)
        },
        removeItem(key) {
          if (storageUnavailable) throw Error('unavailable')
          storage.delete(key)
        }
      },
      window: {
        setTimeout(fn, delay) {
          assert.equal(delay, 250)
          timers.set(++timerId, fn)
          return timerId
        },
        clearTimeout(id) {
          timers.delete(id)
        }
      }
    }
  )
  const render = () => {
    cursor = 0
    const tree = exports.MusicSearch({ pageMode })
    effects.splice(0).forEach((fn) => fn())
    return pageMode ? tree : tree.props.children
  }
  const input = () => render().props.children[0].props.children[1]
  return {
    render,
    input,
    requests,
    navigations,
    storage,
    changePage(value) {
      page = value
      render()
    },
    type(value) {
      if (pageMode) input().props.onChange({ target: { value } })
      else render().props.onInputValueChange(value, { reason: 'input-change', cancel() {} })
      render()
    },
    tick() {
      const pending = [...timers.values()]
      timers.clear()
      pending.forEach((fn) => fn())
    }
  }
}
const settle = async () => {
  await Promise.resolve()
  await Promise.resolve()
}

test('mobile search displays twenty unique history terms, submits them and clears stored history', () => {
  const values = Array.from({ length: 25 }, (_, index) => `term ${index}`)
  const h = harness(JSON.stringify(values), false, true)
  h.render()
  const history = () => h.render().props.children[1]
  assert.equal(history().props.children[1].props.children.length, 20)
  history().props.children[1].props.children[0].props.onClick()
  assert.deepEqual(h.navigations, [['search', 'term 0']])
  h.type('')
  history().props.children[0].props.children[1].props.onClick()
  assert.equal(h.storage.has('nons-search-history'), false)
  assert.equal(history().props.children[1].type, 'p')
})

test('mobile completion is bounded to ten and stale requests cannot replace current suggestions', async () => {
  const h = harness('[]', false, true)
  h.render()
  h.input().props.onFocus()
  h.type('old')
  h.tick()
  h.type('new')
  h.tick()
  h.requests[1].resolve(Array.from({ length: 15 }, (_, index) => `new ${index}`))
  await settle()
  const suggestions = () => h.render().props.children[1].props.children[1]
  assert.equal(suggestions().length, 10)
  h.requests[0].resolve(['old result'])
  await settle()
  assert.equal(suggestions()[0].props.children[1].props.children, 'new 0')
  suggestions()[9].props.onClick()
  assert.deepEqual(h.navigations, [['search', 'new 9']])
})

test('popup dismissal preserves input before and after submitting search', () => {
  const h = harness()
  h.render()
  h.input().props.onFocus()
  h.type('song')
  function dismiss() {
    h.input().props.onBlur()
    h.render().props.onOpenChange(false)
    let cancelled = false
    h.render().props.onInputValueChange('', {
      reason: 'input-clear',
      cancel() {
        cancelled = true
      }
    })
    assert.equal(h.render().props.inputValue, 'song')
    assert.equal(cancelled, true)
    assert.equal(h.render().props.open, false)
  }
  dismiss()
  h.render().props.children[0].props.onSubmit({ preventDefault() {} })
  assert.deepEqual(h.navigations, [['search', 'song']])
  dismiss()
  h.changePage({ view: 'search', query: 'song' })
  dismiss()
  h.changePage({ view: 'library', query: '' })
  assert.equal(h.render().props.inputValue, 'song')
  h.type('')
  assert.equal(h.render().props.inputValue, '')
})

test('selecting a suggestion preserves its keyword when the popup closes', () => {
  const h = harness()
  h.render()
  h.type('partial')
  h.render().props.onValueChange('selected song')
  h.render().props.onInputValueChange('', { reason: 'input-clear', cancel() {} })
  assert.equal(h.render().props.inputValue, 'selected song')
  assert.deepEqual(h.navigations, [['search', 'selected song']])
})

test('empty focused input shows persisted history and selecting it searches again', () => {
  const h = harness(JSON.stringify(['recent', 'older']))
  h.render()
  assert.equal(h.render().props.open, false)
  h.input().props.onFocus()
  assert.equal(h.render().props.open, true)
  assert.deepEqual([...h.render().props.items], ['recent', 'older'])
  h.tick()
  assert.equal(h.requests.length, 0)
  h.render().props.onValueChange('older')
  assert.deepEqual(h.navigations, [['search', 'older']])
  h.type('')
  h.input().props.onFocus()
  assert.deepEqual([...h.render().props.items], ['older', 'recent'])
  h.render().props.onOpenChange(false)
  assert.equal(h.render().props.open, false)
  h.input().props.onBlur()
  h.render()
  h.input().props.onFocus()
  assert.equal(h.render().props.open, true)
})

test('submitted history keeps ten unique recent terms and survives remount', () => {
  const h = harness()
  h.render()
  const submit = (value) => {
    h.type(value)
    h.render().props.children[0].props.onSubmit({ preventDefault() {} })
  }
  submit('   ')
  for (let i = 0; i < 12; i++) submit(`song ${i}`)
  submit(' song 5 ')
  const expected = [
    'song 5',
    'song 11',
    'song 10',
    'song 9',
    'song 8',
    'song 7',
    'song 6',
    'song 4',
    'song 3',
    'song 2'
  ]
  assert.deepEqual(JSON.parse(h.storage.get('nons-search-history')), expected)
  const restored = harness(h.storage.get('nons-search-history'))
  restored.render()
  assert.deepEqual([...restored.render().props.items], expected)
  h.changePage({ view: 'local', query: '' })
  submit('local song')
  assert.deepEqual(JSON.parse(h.storage.get('nons-search-history')), expected)
  h.type('')
  h.input().props.onFocus()
  assert.equal(h.render().props.open, false)
})

test('invalid or unavailable storage does not prevent search', () => {
  for (const saved of [
    'invalid json',
    '{}',
    JSON.stringify([null, 42, '', '  ', 'x'.repeat(86)])
  ]) {
    const h = harness(saved)
    h.render()
    h.input().props.onFocus()
    assert.equal(h.render().props.items.length, 0)
    assert.equal(h.render().props.open, false)
  }
  const h = harness('[]', true)
  h.render()
  h.type('song')
  h.render().props.children[0].props.onSubmit({ preventDefault() {} })
  assert.deepEqual(h.navigations, [['search', 'song']])
  h.type('')
  h.input().props.onFocus()
  assert.deepEqual([...h.render().props.items], ['song'])
})

test('clearing input restores history and discards in-flight suggestions', async () => {
  const h = harness('["recent"]')
  h.render()
  h.input().props.onFocus()
  h.type('query')
  h.tick()
  h.type('')
  h.requests[0].resolve(['late'])
  await settle()
  assert.deepEqual([...h.render().props.items], ['recent'])
  assert.equal(h.render().props.open, true)
})

test('suggestions debounce typing, ignore older responses and display at most five', async () => {
  const h = harness()
  h.render()
  h.input().props.onFocus()
  h.type('first')
  h.type('second')
  assert.equal(h.requests.length, 0)
  h.tick()
  assert.equal(h.requests.length, 1)
  assert.equal(h.requests[0].args.keyword, 'second')
  h.type('third')
  h.tick()
  h.requests[0].resolve(['stale'])
  await settle()
  assert.equal(h.render().props.items.length, 0)
  h.requests[1].resolve(['a', 'b', 'c', 'd', 'e', 'f'])
  await settle()
  assert.deepEqual([...h.render().props.items], ['a', 'b', 'c', 'd', 'e'])
  assert.equal(h.render().props.open, true)
  h.render().props.onValueChange('b')
  assert.deepEqual(h.navigations, [['search', 'b']])
})

test('composition and local search skip suggestions; failed suggestions still allow direct submit', async () => {
  const h = harness()
  h.render()
  h.input().props.onFocus()
  h.input().props.onCompositionStart()
  h.type('中')
  h.tick()
  assert.equal(h.requests.length, 0)
  h.render().props.children[0].props.onSubmit({ preventDefault() {} })
  assert.equal(h.navigations.length, 0)
  h.input().props.onCompositionEnd()
  h.render()
  h.tick()
  assert.equal(h.requests.length, 1)
  h.requests[0].reject(Error('offline'))
  await settle()
  h.render().props.children[0].props.onSubmit({ preventDefault() {} })
  assert.deepEqual(h.navigations, [['search', '中']])
  h.changePage({ view: 'local', query: 'local' })
  h.type('local music')
  h.tick()
  assert.equal(h.requests.length, 1)
  h.render().props.children[0].props.onSubmit({ preventDefault() {} })
  assert.deepEqual(h.navigations[1], ['local', 'local music'])
})

test('blur drops in-flight suggestions and prevents the popup from reopening', async () => {
  const h = harness()
  h.render()
  h.input().props.onFocus()
  h.type('song')
  h.tick()
  h.input().props.onBlur()
  h.render()
  h.requests[0].resolve(['late'])
  await settle()
  assert.equal(h.render().props.open, false)
  assert.equal(h.render().props.items.length, 0)
})
