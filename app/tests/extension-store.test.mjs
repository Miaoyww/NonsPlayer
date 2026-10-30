import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import React, { act } from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { renderToStaticMarkup } from 'react-dom/server'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(name, modules = {}) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(new URL(`../src/features/settings/extensions/${name}`, import.meta.url), 'utf8'),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    { exports, require: (name) => modules[name] }
  )
  return exports
}
const model = load('extension-store-state.ts')
const card = load('extension-store-card.tsx', {
  react: React,
  'react/jsx-runtime': jsx,
  '@tauri-apps/api/core': { isTauri: () => false },
  '@tauri-apps/plugin-opener': { openUrl: async () => {} },
  'lucide-react': Object.fromEntries(
    ['ArrowRight', 'Clock3', 'Download', 'UserRound'].map((name) => [name, 'svg'])
  ),
  '@/components/ui/button': {
    Button: ({ variant, size, ...props }) => React.createElement('button', props)
  },
  '@/components/error-notice': { ErrorNotice: () => null },
  '@/lib/player': { errorText: String },
  '@/assets/icons/github.svg': { default: 'github.svg' }
})
const entry = (overrides = {}) => ({
  id: 'download',
  name: '下载管理',
  version: '2.0.0',
  description: '保存音乐',
  labels: ['download'],
  authors: [{ name: 'Miaoyww' }],
  license: 'MIT',
  compatible: true,
  updateAvailable: false,
  bundled: false,
  artifact: { url: 'https://github.com/example/download/releases/download/v2/plugin.zip' },
  permissions: ['account:credentials'],
  httpHosts: ['music.example.org'],
  ...overrides
})
const index = (source = 'cdn', entries = [entry()]) => ({
  source,
  url: `https://${source}.example/index.json`,
  generatedAt: '2026-10-30',
  plugins: entries,
  adapters: entries
})
const preview = (token = 'token', overrides = {}) => ({
  token,
  kind: 'plugin',
  id: 'download',
  name: '下载管理',
  version: '2.0.0',
  permissions: ['读取账号凭据'],
  warnings: ['请仅信任可靠作者'],
  ...overrides
})
const flush = async () => {
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
}

function harness(kind = 'plugin') {
  const calls = [],
    busy = [],
    reports = [],
    installed = []
  const controller = new model.ExtensionStoreController(
    kind,
    (command, args) =>
      new Promise((resolve, reject) => calls.push({ command, args, resolve, reject })),
    {
      onBusyChange: (value) => busy.push(value),
      onInstalled: () => installed.push('installed'),
      report: (cause) => reports.push(cause),
      errorText: (cause) => String(cause)
    }
  )
  controller.activate()
  return { controller, calls, busy, reports, installed }
}

test('source switch rejects old index, detail and late prepare, and always refreshes source cache', async () => {
  const { controller: c, calls, busy } = harness()
  assert.equal(calls[0].args.refresh, true)
  assert.equal('source' in calls[0].args, false)
  c.load('github')
  c.load('github')
  assert.equal(calls.length, 2, 'duplicate index calls coalesce')
  calls[1].resolve(index('github'))
  await flush()
  calls[0].resolve(index('cdn', [entry({ name: '旧索引' })]))
  await flush()
  assert.equal(c.getSnapshot().index.source, 'github')
  const selected = c.entries()[0]
  c.select(selected)
  c.prepare(selected, 'github')
  c.prepare(selected, 'github')
  assert.equal(calls.length, 3)
  assert.equal(calls[2].args.source, 'github')
  c.load('cdn')
  assert.equal(c.getSnapshot().detail, undefined)
  assert.equal(c.getSnapshot().review, undefined)
  calls[2].resolve(preview('stale'))
  await flush()
  assert.equal(calls.at(-1).command, 'extension_store_cancel')
  assert.equal(calls.at(-1).args.token, 'stale')
  assert.equal(c.getSnapshot().review, undefined)
  calls[3].resolve(index())
  await flush()
  assert.deepEqual(busy, [true, false])
  c.load('cdn')
  assert.equal(calls.at(-1).args.source, 'cdn')
  assert.equal(calls.at(-1).args.refresh, true)
  c.dispose()
})

test('authorization gates install, duplicate installs are blocked, parent callback never waits for index refresh', async () => {
  const { controller: c, calls, busy, installed } = harness()
  calls[0].resolve(index())
  await flush()
  c.prepare(c.entries()[0], 'cdn')
  calls[1].resolve(preview())
  await flush()
  await c.install()
  assert.equal(calls.length, 2, 'review alone does not authorize install')
  assert.deepEqual(busy, [true], 'busy spans prepare and the entire review')
  c.setTrusted(true)
  const installing = c.install()
  c.install()
  c.load('github')
  assert.equal(calls.length, 3)
  assert.equal(calls[2].command, 'extension_store_install')
  assert.equal(calls[2].args.confirmed, true)
  calls[2].resolve()
  await flush()
  assert.equal(calls[3].command, 'extension_store_index')
  assert.equal(calls[3].args.source, 'cdn')
  assert.equal(calls[3].args.refresh, true)
  await installing
  assert.deepEqual(installed, ['installed'])
  assert.equal(c.getSnapshot().notice, '安装成功，可在已安装中启用')
  assert.equal(c.getSnapshot().loading, true, 'parent notified while refresh is still pending')
  c.load('github')
  calls[3].resolve(index('cdn', [entry({ installedVersion: '2.0.0' })]))
  await flush()
  assert.deepEqual(installed, ['installed'], 'stale refresh cannot swallow or repeat success')
  calls[4].resolve(index('github'))
  await flush()
  assert.deepEqual(busy, [true, false])
  assert.equal(
    calls.some(({ command }) => command === 'plugin_action'),
    false
  )
  assert.equal(
    calls.some(({ command }) => command === 'extension_store_cancel'),
    false
  )
  c.dispose()
})

test('success notices distinguish fresh plugins, upgrades and loaded adapters even if refresh fails', async () => {
  for (const [kind, installedVersion, notice] of [
    ['plugin', undefined, '安装成功，可在已安装中启用'],
    ['plugin', '1.0.0', '升级成功，已保留原启用状态'],
    ['adapter', undefined, '安装成功，适配器已加载'],
    ['adapter', '1.0.0', '升级成功，适配器已加载']
  ]) {
    const { controller: c, calls, installed } = harness(kind)
    calls[0].resolve(index())
    await flush()
    c.prepare(c.entries()[0], 'cdn')
    calls[1].resolve(preview('success', { kind, installedVersion }))
    await flush()
    c.setTrusted(true)
    const installing = c.install()
    calls[2].resolve()
    await installing
    assert.deepEqual(installed, ['installed'])
    assert.equal(c.getSnapshot().notice, notice)
    calls[3].reject('index offline')
    await flush()
    assert.deepEqual(installed, ['installed'])
    assert.equal(c.getSnapshot().notice, notice)
    assert.match(c.getSnapshot().error, /index offline/)
    c.dispose()
  }
})

test('cancel and disposal release preview tokens including prepare resolving after unmount', async () => {
  const { controller: c, calls } = harness()
  calls[0].resolve(index())
  await flush()
  c.prepare(c.entries()[0], 'cdn')
  calls[1].resolve(preview('review'))
  await flush()
  c.closeReview()
  assert.equal(calls[2].args.token, 'review')
  c.closeReview()
  assert.equal(calls.length, 3, 'cancel is idempotent')
  c.prepare(c.entries()[0], 'cdn')
  c.dispose()
  calls[3].resolve(preview('late'))
  await flush()
  assert.equal(calls[4].command, 'extension_store_cancel')
  assert.equal(calls[4].args.token, 'late')
  assert.equal(c.getSnapshot().review, undefined)
  const second = harness()
  second.calls[0].resolve(index())
  await flush()
  second.controller.prepare(second.controller.entries()[0], 'cdn')
  second.calls[1].resolve(preview('mounted'))
  await flush()
  second.controller.dispose()
  assert.equal(second.calls[2].args.token, 'mounted')
})

test('source and bundled entries never prepare, backend installation status wins over parent hints', async () => {
  const { controller: c, calls } = harness('adapter')
  const entries = [
    entry({ distribution: 'source', installable: false }),
    entry({ id: 'official', distribution: 'source', bundled: true }),
    entry({ id: 'current', installedVersion: '2.0.0' }),
    entry({ id: 'blocked', blocked: true }),
    entry({ id: 'incompatible', compatible: false }),
    entry({ id: 'missing-artifact', artifact: null, unavailableReason: '尚未发布安装包' })
  ]
  calls[0].resolve(index('cdn', entries))
  await flush()
  for (const item of c.entries()) await c.prepare(item, 'cdn')
  assert.equal(calls.length, 1)
  assert.match(model.unavailableReason(entries[0]), /作者尚未完成/)
  assert.equal(model.unavailableReason(entries[1]), '官方已随包安装')
  assert.equal(model.unavailableReason(entries[5]), '尚未发布安装包')
  c.dispose()
})

test('index and package failures preserve content, can retry, and never install a missing real package', async () => {
  const { controller: c, calls } = harness()
  calls[0].resolve(index())
  await flush()
  c.select(c.entries()[0])
  c.load('cdn')
  calls[1].reject('offline')
  await flush()
  assert.equal(c.entries().length, 1)
  assert.equal(c.getSnapshot().detail.entry.id, 'download')
  c.prepare(c.entries()[0], 'cdn')
  calls[2].reject('GitHub Release 404')
  await flush()
  assert.match(c.getSnapshot().error, /下载或校验安装包失败.*404/)
  assert.equal(c.getSnapshot().busy, false)
  c.prepare(c.entries()[0], 'cdn')
  calls[3].resolve(preview('retry'))
  await flush()
  c.setTrusted(true)
  c.install()
  calls[4].reject('validation failed')
  await flush()
  assert.equal(calls[5].command, 'extension_store_cancel')
  assert.match(c.getSnapshot().error, /安装失败，请重试/)
  c.prepare(c.entries()[0], 'cdn')
  assert.equal(calls[6].command, 'extension_store_prepare')
  assert.equal(c.getSnapshot().trusted, false, 'retry requires fresh authorization')
  c.dispose()
  calls[6].resolve(preview('cleanup'))
  await flush()
})

test('strict effect remount and obsolete failures cannot overwrite current state', async () => {
  const { controller: c, calls } = harness()
  c.dispose()
  c.activate()
  calls[0].reject('old error')
  calls[1].resolve(index('github'))
  await flush()
  assert.equal(c.getSnapshot().error, undefined)
  assert.equal(c.getSnapshot().index.source, 'github')
  assert.equal(c.getSnapshot().loading, false)
  c.dispose()
})

test('switching away from an authorized review cancels it and new source requires fresh trust', async () => {
  const { controller: c, calls } = harness()
  calls[0].resolve(index())
  await flush()
  c.select(c.entries()[0])
  c.prepare(c.entries()[0], 'cdn')
  calls[1].resolve(preview('old-review'))
  await flush()
  c.setTrusted(true)
  c.load('github')
  assert.equal(calls[2].command, 'extension_store_cancel')
  assert.equal(calls[2].args.token, 'old-review')
  assert.equal(c.getSnapshot().trusted, false)
  assert.equal(c.getSnapshot().detail, undefined)
  await c.install()
  assert.equal(calls.length, 4)
  calls[3].resolve(index('github'))
  await flush()
  c.prepare(c.entries()[0], 'cdn')
  assert.equal(calls.length, 4, 'old source cannot prepare new entries')
  c.prepare(c.entries()[0], 'github')
  calls[4].resolve(preview('new-review'))
  await flush()
  await c.install()
  assert.equal(calls.length, 5, 'new review is not trusted')
  c.dispose()
})

test('unmount during install cancels retained token and prevents stale completion callbacks', async () => {
  const { controller: c, calls, busy, installed } = harness()
  calls[0].resolve(index())
  await flush()
  c.prepare(c.entries()[0], 'cdn')
  calls[1].resolve(preview('installing'))
  await flush()
  c.setTrusted(true)
  const installing = c.install()
  c.dispose()
  assert.equal(calls[3].command, 'extension_store_cancel')
  assert.equal(calls[3].args.token, 'installing')
  calls[2].resolve()
  await installing
  assert.deepEqual(installed, [])
  assert.deepEqual(busy, [true, false])
  assert.equal(calls.length, 4)
})

test('search covers name, id, description and labels; permission domains survive formatting', () => {
  const entries = [entry()]
  for (const query of ['下载', 'DOWNLOAD', '保存音乐', 'download 保存'])
    assert.equal(model.searchEntries(entries, query).length, 1)
  assert.equal(model.searchEntries(entries, 'missing').length, 0)
  assert.match(
    model.permissionLines(entries[0]).join(' '),
    /account:credentials.*music.example.org/
  )
  assert.match(
    model
      .permissionLines(
        entry({ permissions: { httpHosts: ['api.example.org'], credentials: true } })
      )
      .join(' '),
    /api.example.org.*credentials/
  )
})

test('plugin and adapter cards display total downloads with k units and exact accessible counts', () => {
  const cases = [
    [0, '0'],
    [999, '999'],
    [1000, '1k'],
    [1049, '1k'],
    [1050, '1.1k'],
    [1150, '1.2k'],
    [1250, '1.3k'],
    [1999, '2k'],
    [10000, '10k'],
    [1000000, '1000k'],
    [undefined, '暂无统计'],
    [null, '暂无统计'],
    [-1, '暂无统计'],
    [1.5, '暂无统计'],
    ['1250', '暂无统计'],
    [NaN, '暂无统计'],
    [Infinity, '暂无统计'],
    [Number.MAX_SAFE_INTEGER + 1, '暂无统计']
  ]
  for (const kind of ['plugin', 'adapter']) {
    for (const [count, expected] of cases) {
      const dom = new JSDOM(
        renderToStaticMarkup(
          React.createElement(card.ExtensionStoreCard, {
            entry: entry({ kind, total_download_count: count }),
            busy: false,
            onSelect: () => {},
            action: null
          })
        )
      )
      try {
        const downloads = dom.window.document.querySelector('[title^="下载总量："]')
        assert.equal(downloads.textContent, expected, `${kind}: ${String(count)}`)
        const label = `下载总量：${expected === '暂无统计' ? expected : count.toLocaleString('zh-CN')}`
        assert.equal(downloads.title, label)
        assert.equal(downloads.getAttribute('aria-label'), label)
      } finally {
        dom.window.close()
      }
    }
  }
})

test('React store renders bounded searchable list, index source, details and explicit trust dialog', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const calls = [],
    busy = []
  let loadMore
  const wrapper = ({ children }) => React.createElement('div', null, children)
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    './extension-store-card': card,
    './extension-store-state': model,
    '@/components/music/infinite-load': {
      InfiniteLoad: ({ onLoad }) => {
        loadMore = onLoad
        return React.createElement('span', { 'data-infinite-marker': true })
      }
    },
    'lucide-react': Object.fromEntries(
      ['Download', 'LoaderCircle', 'Package', 'RefreshCw', 'Search', 'Store'].map((name) => [
        name,
        'svg'
      ])
    ),
    '@/components/ui/button': {
      Button: ({ variant, size, ...props }) => React.createElement('button', props)
    },
    '@/components/ui/input': {
      Input: ({ onChange, ...props }) =>
        React.createElement('input', { ...props, onInput: onChange })
    },
    '@/components/ui/select': Object.fromEntries(
      ['Select', 'SelectContent', 'SelectItem', 'SelectTrigger', 'SelectValue'].map((name) => [
        name,
        wrapper
      ])
    ),
    '@/components/ui/dialog': {
      Dialog: ({ open, children }) =>
        open ? React.createElement('div', { role: 'dialog' }, children) : null,
      ...Object.fromEntries(
        ['DialogContent', 'DialogDescription', 'DialogFooter', 'DialogHeader', 'DialogTitle'].map(
          (name) => [name, wrapper]
        )
      )
    },
    '@/components/error-notice': {
      ErrorNotice: ({ error }) => error && React.createElement('p', { role: 'alert' }, error)
    },
    '@/components/music/loading': {
      TextSkeleton: () => React.createElement('div', { role: 'status' }, '正在加载')
    },
    '@/lib/player': {
      errorText: String,
      nativeCall: (command, args) =>
        new Promise((resolve, reject) => calls.push({ command, args, resolve, reject }))
    },
    '@/lib/utils': {
      cn: (...values) => values.filter(Boolean).join(' ')
    }
  }
  const { ExtensionStore } = load('extension-store.tsx', modules)
  const root = createRoot(document.getElementById('root'))
  const button = (label, scope = document) =>
    [...scope.querySelectorAll('button')].find((node) => node.textContent === label)
  const click = async (label, scope) =>
    act(async () => {
      const node = button(label, scope)
      assert.ok(node, label)
      node.click()
    })
  try {
    await act(async () =>
      root.render(
        React.createElement(ExtensionStore, {
          kind: 'plugin',
          onInstalled: () => {},
          onBusyChange: (value) => busy.push(value),
          installed: [{ id: 'download', version: '2.0.0' }]
        })
      )
    )
    await act(async () =>
      calls[0].resolve(
        index('cdn', [
          entry(),
          ...Array.from({ length: 30 }, (_, i) => entry({ id: `item-${i}`, name: `条目 ${i}` }))
        ])
      )
    )
    assert.equal(document.querySelectorAll('article').length, 24)
    assert.equal(button('显示更多'), undefined)
    assert.ok(document.querySelector('[data-infinite-marker]'))
    await act(async () => loadMore())
    assert.equal(document.querySelectorAll('article').length, 31)
    const search = document.querySelector('input')
    await act(async () => {
      Object.getOwnPropertyDescriptor(dom.window.HTMLInputElement.prototype, 'value').set.call(
        search,
        '保存音乐 download'
      )
      search.dispatchEvent(new dom.window.Event('input', { bubbles: true }))
    })
    await click('下载管理')
    const detail = document.querySelector('[role="dialog"]')
    assert.match(
      detail.textContent,
      /Miaoyww.*MIT.*GitHub Release.*account:credentials.*music.example.org/
    )
    assert.match(document.body.textContent, /https:\/\/cdn.example\/index.json/)
    assert.equal(document.querySelectorAll('img').length, 0)
    await click('安装', detail)
    assert.deepEqual(busy, [true])
    const activeArticle = document.querySelector('article')
    assert.ok(button('正在下载安装包…', activeArticle))
    for (const article of [...document.querySelectorAll('article')].slice(1)) {
      assert.equal(button('正在下载安装包…', article), undefined)
      assert.equal(button('安装', article).disabled, true)
    }
    await act(async () => calls[1].resolve(preview()))
    assert.equal(button('正在下载安装包…', activeArticle), undefined)
    assert.equal(button('确认安装').disabled, true)
    assert.match(
      document.querySelector('[role="dialog"]').textContent,
      /读取账号凭据.*请仅信任可靠作者/
    )
    await act(async () => document.querySelector('input[type=checkbox]').click())
    assert.equal(button('确认安装').disabled, false)
    await click('取消')
    assert.equal(calls.at(-1).command, 'extension_store_cancel')
    assert.deepEqual(busy, [true, false])
    assert.equal(
      calls.some(({ command }) => command === 'extension_store_install'),
      false
    )
    await click('安装', document.querySelector('[role="dialog"]'))
    await act(async () => calls.at(-1).resolve(preview('upgrade', { installedVersion: '1.0.0' })))
    const upgrade = document.querySelector('[role="dialog"]')
    assert.match(upgrade.textContent, /确认升级插件/)
    assert.match(upgrade.textContent, /同意升级/)
    assert.ok(button('确认升级'))
    assert.equal(button('确认安装'), undefined)
    await click('取消')
    assert.ok(
      [...document.querySelectorAll('svg')].every(
        (node) => node.getAttribute('aria-hidden') === 'true'
      )
    )
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
