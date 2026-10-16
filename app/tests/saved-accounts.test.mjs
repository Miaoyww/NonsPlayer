import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import React, { act } from 'react'
import { createRoot } from 'react-dom/client'
import * as jsx from 'react/jsx-runtime'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

test('saved accounts select references, confirm deletion and ignore responses after closing', async () => {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = {
    window: globalThis.window,
    document: globalThis.document,
    act: globalThis.IS_REACT_ACT_ENVIRONMENT
  }
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const requests = []
  const request = (operation, reference) =>
    new Promise((resolve, reject) => requests.push({ operation, reference, resolve, reject }))
  const modules = {
    '@/features/music/source-preferences': {
      sourcePreferences: { beginIntent: async () => 1, use: async () => {} }
    },
    react: React,
    'react/jsx-runtime': jsx,
    'lucide-react': {
      Check: () => null,
      RefreshCw: () => null,
      Trash2: () => null,
      UserRound: () => null
    },
    '@/components/music/action-button': {
      ActionButton: ({ children, variant, size, ...props }) =>
        React.createElement('button', props, children)
    },
    '@/lib/player': { errorText: String },
    './account-avatar': {
      AccountAvatar: ({ avatar }) => React.createElement('img', { src: avatar, alt: '' })
    },
    '@/features/music/client': {
      musicClient: {
        accounts: (source) => request('accounts', source),
        selectAccount: (reference) => request('select', reference),
        removeAccount: (reference) => request('remove', reference)
      }
    }
  }
  const exports = {}
  const code = ts.transpileModule(
    readFileSync(new URL('../src/features/account/saved-accounts.tsx', import.meta.url), 'utf8'),
    {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }
  ).outputText
  runInNewContext(code, {
    exports,
    require: (name) => (name === '@/components/error-notice' ? errorNoticeModule : modules[name])
  })
  const root = createRoot(document.getElementById('root'))
  const render = (open, source = 'netease', currentSource = source) =>
    act(async () =>
      root.render(
        React.createElement(exports.SavedAccounts, {
          open,
          source,
          sourceName: '网易云',
          current: { source: currentSource, id: '1' }
        })
      )
    )
  const click = (button) =>
    act(async () => button.dispatchEvent(new dom.window.MouseEvent('click', { bubbles: true })))
  const button = (label) =>
    [...document.querySelectorAll('button')].find(
      (item) => item.textContent === label + '网易云' || item.textContent === label
    )
  const records = [1, 2].map((id) => ({
    reference: { source: 'netease', id: String(id) },
    displayName: `User ${id}`,
    avatar: `https://example.com/avatar-${id}.jpg`
  }))
  try {
    await render(false)
    assert.equal(requests.length, 0)
    await render(true)
    assert.equal(requests[0].reference, 'netease')
    await act(async () => requests[0].resolve(records))
    assert.equal(
      button('User 1').disabled,
      false,
      'selected accounts can switch the workspace source again'
    )
    assert.equal(button('User 1').querySelector('img').src, records[0].avatar)
    await click(button('User 2'))
    assert.equal(requests[1].operation, 'select')
    assert.deepEqual(JSON.parse(JSON.stringify(requests[1].reference)), records[1].reference)
    assert.equal(button('User 2').disabled, true)
    await act(async () => requests[1].reject(new Error('selection failed')))
    assert.match(document.querySelector('[role=alert]').textContent, /selection failed/)
    await click(document.querySelector('[aria-label="删除保存的账号 User 2"]'))
    assert.equal(requests.length, 2, 'opening confirmation must not delete')
    await click(button('取消'))
    assert.equal(requests.length, 2)
    await click(document.querySelector('[aria-label="删除保存的账号 User 2"]'))
    await click(button('确定删除'))
    assert.equal(requests[2].operation, 'remove')
    assert.deepEqual(JSON.parse(JSON.stringify(requests[2].reference)), records[1].reference)
    await act(async () => requests[2].resolve())
    assert.equal(requests[3].operation, 'accounts')
    await render(false)
    await act(async () => requests[3].resolve(records))
    assert.equal(document.body.textContent, '')
    await render(true)
    await act(async () => requests[4].resolve([records[0]]))
    assert.ok(button('User 1'))
    assert.equal(button('User 2'), undefined)
    await render(true, 'other', 'netease')
    assert.equal(requests[5].reference, 'other')
    const other = { ...records[0], reference: { source: 'other', id: '1' } }
    await act(async () => requests[5].resolve([other]))
    assert.equal(
      button('User 1').disabled,
      false,
      'matching IDs from different sources are separate accounts'
    )
    await click(button('User 1'))
    assert.equal(requests[6].reference.source, 'other')
    await render(false)
    await act(async () => requests[6].resolve())
    assert.equal(document.body.textContent, '')
  } finally {
    await act(async () => root.unmount())
    dom.window.close()
    globalThis.window = previous.window
    globalThis.document = previous.document
    globalThis.IS_REACT_ACT_ENVIRONMENT = previous.act
  }
})
