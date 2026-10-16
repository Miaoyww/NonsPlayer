import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import ts from 'typescript'

function load(path, modules, globals = {}) {
  const exports = {}
  const { outputText } = ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  })
  runInNewContext(outputText, { exports, require: (name) => modules[name], ...globals })
  return exports
}

function hooks() {
  const slots = [],
    effects = []
  let cursor = 0
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
    useMemo(fn, deps) {
      const i = cursor++,
        old = slots[i]
      if (!old || deps.some((v, j) => v !== old.deps[j])) slots[i] = { deps, value: fn() }
      return slots[i].value
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
  }
  return {
    react,
    render(fn) {
      cursor = 0
      const result = fn()
      while (effects.length) effects.shift()()
      return result
    },
    dispose() {
      for (const slot of slots) slot?.cleanup?.()
    }
  }
}
const jsx = (type, props) => ({ type, props })
const jsxRuntime = { jsx, jsxs: jsx }
const settle = async () => {
  for (let i = 0; i < 5; i++) await Promise.resolve()
}

function quoteHarness() {
  const state = hooks()
  const requests = []
  const { GreetingQuote } = load('../src/features/discovery/greeting-quote.tsx', {
    react: state.react,
    'react/jsx-runtime': jsxRuntime,
    '@/lib/player': {
      nativeCall: (command) =>
        new Promise((resolve, reject) => requests.push({ command, resolve, reject }))
    },
    '@/components/magicui/typing-animation': { TypingAnimation: 'typing' }
  })
  return { requests, render: () => state.render(GreetingQuote), dispose: state.dispose }
}

test('successful one-line quote uses fast typing and reserves its final layout', async () => {
  const app = quoteHarness()
  assert.equal(app.render().props.children[0].props.children, '由此开启好心情 ～')
  assert.equal(app.requests[0].command, 'discovery_hitokoto')
  app.requests[0].resolve(' 音乐相伴 ')
  await settle()
  const children = app.render().props.children
  assert.equal(children[0].props.children, '音乐相伴')
  assert.equal(children[1].props.children, '音乐相伴')
  assert.equal(children[2].props.duration, 85)
  assert.equal(children[2].props['aria-hidden'], 'true')
})

test('errors and empty content use the requested fallback; late unmounted responses are discarded', async () => {
  for (const outcome of ['error', 'empty']) {
    const app = quoteHarness()
    app.render()
    if (outcome === 'error') app.requests[0].reject(new Error('timeout'))
    else app.requests[0].resolve(' ')
    await settle()
    assert.equal(app.render().props.children[2].props.children, '由此开启好心情 ～')
  }
  const app = quoteHarness()
  app.render()
  app.dispose()
  app.requests[0].resolve('late quote')
  await settle()
  assert.equal(app.render().props.children[0].props.children, '由此开启好心情 ～')
})

test('Magic UI types whole Unicode code points, hides its finished cursor and skips timers for reduced motion', () => {
  for (const reduced of [false, true]) {
    const state = hooks(),
      timers = new Map()
    let serial = 0
    const { TypingAnimation } = load(
      '../src/components/magicui/typing-animation.tsx',
      {
        react: state.react,
        'react/jsx-runtime': jsxRuntime,
        'motion/react': {
          motion: { span: 'span' },
          useInView: () => true,
          useReducedMotion: () => reduced
        },
        '@/lib/utils': { cn: (...names) => names.filter(Boolean).join(' ') }
      },
      {
        setTimeout(fn) {
          const id = ++serial
          timers.set(id, fn)
          return id
        },
        clearTimeout(id) {
          timers.delete(id)
        }
      }
    )
    const render = () => state.render(() => TypingAnimation({ children: '好🎵', duration: 35 }))
    let tree = render()
    if (reduced) {
      assert.equal(tree.props.children[0], '好🎵')
      assert.equal(tree.props.children[1], false)
      assert.equal(timers.size, 0)
    } else {
      const tick = () => {
        const [id, fn] = timers.entries().next().value
        timers.delete(id)
        fn()
        return render()
      }
      tree = tick()
      assert.equal(tree.props.children[0], '好')
      tree = tick()
      assert.equal(tree.props.children[0], '好🎵')
      assert.equal(tree.props.children[1], false)
      tick()
      assert.equal(timers.size, 0)
    }
    state.dispose()
    assert.equal(timers.size, 0)
  }
})
