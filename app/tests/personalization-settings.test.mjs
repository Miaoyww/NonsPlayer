import { errorNoticeModule } from './helpers/error-notice.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

const compile = (path) =>
  ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
  }).outputText

const taskbarCompiled = compile('../src/features/settings/appearance/taskbar-settings.tsx')
const pageCompiled = compile('../src/features/settings/pages/personalization.tsx')

async function harness({ platform = 'Windows', persist = async () => {} } = {}) {
  const dom = new JSDOM('<div id="root"></div>')
  const previous = Object.fromEntries(
    ['window', 'document', 'IS_REACT_ACT_ENVIRONMENT'].map((key) => [
      key,
      Object.getOwnPropertyDescriptor(globalThis, key)
    ])
  )
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  let state = {
    playbackOptions: {
      fades: false,
      crossfade: false,
      crossfadeDurationMs: 3000,
      taskbarCover: false,
      autoplay: false
    }
  }
  const listeners = new Set()
  const calls = []
  const diagnostics = []
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/platform': { isWindows: () => platform === 'Windows' },
    '@/lib/diagnostics': {
      reportCaughtError: (operation, cause) => diagnostics.push({ operation, cause })
    },
    '@/components/error-notice': errorNoticeModule,
    '@/components/ui/switch': {
      Switch: ({ onCheckedChange, ...props }) =>
        React.createElement('input', {
          ...props,
          type: 'checkbox',
          onChange: (event) => onCheckedChange(event.target.checked)
        })
    },
    '@/features/settings/shell/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement(
          'section',
          { 'data-title': title },
          React.createElement('p', {}, description),
          children
        )
    },
    '@/lib/player': {
      usePlayer: () =>
        React.useSyncExternalStore(
          (listener) => {
            listeners.add(listener)
            return () => listeners.delete(listener)
          },
          () => state
        ),
      errorText: (error) => error.message,
      nativeCall: async (command, args) => {
        calls.push([command, args])
        assert.equal(command, 'set_playback_options')
        await persist(command, args)
        state = { ...state, playbackOptions: args.options }
        listeners.forEach((listener) => listener())
      }
    }
  }
  const taskbarExports = {}
  runInNewContext(taskbarCompiled, {
    exports: taskbarExports,
    require(name) {
      const module = modules[name]
      assert.ok(module, `unexpected dependency: ${name}`)
      return module
    }
  })
  const pageExports = {}
  runInNewContext(pageCompiled, {
    exports: pageExports,
    require(name) {
      if (name === '@/features/settings/appearance/taskbar-settings') return taskbarExports
      if (name === '@/features/settings/appearance/personalization-controls')
        return {
          PersonalizationControls: () => React.createElement('div', { 'data-controls': '' })
        }
      const module = modules[name]
      assert.ok(module, `unexpected dependency: ${name}`)
      return module
    }
  })
  const root = createRoot(document.getElementById('root'))
  const card = (title) => document.querySelector(`[data-title="${title}"]`)
  const input = (label) =>
    [...document.querySelectorAll('label')]
      .find((element) => element.textContent.includes(label))
      .querySelector('input')
  await React.act(async () =>
    root.render(
      React.createElement(pageExports.PersonalizationPage, {
        theme: 'dark',
        onThemeChange: () => {}
      })
    )
  )
  return {
    card,
    input,
    calls,
    diagnostics,
    async dispose() {
      await React.act(async () => root.unmount())
      dom.window.close()
      for (const [key, descriptor] of Object.entries(previous)) {
        if (descriptor) Object.defineProperty(globalThis, key, descriptor)
        else delete globalThis[key]
      }
    }
  }
}

test('taskbar cover is grouped under the personalization taskbar heading', async () => {
  const app = await harness()
  try {
    const headings = [...document.querySelectorAll('section[aria-labelledby] > h3')].map(
      (heading) => heading.textContent
    )
    assert.deepEqual(headings, ['任务栏'])
    const section = document.getElementById('personalization-taskbar-heading').closest('section')
    assert.ok(section.querySelector('[data-title="任务栏封面"]'))
    assert.ok(document.querySelector('[data-controls]'))
  } finally {
    await app.dispose()
  }
})

test('taskbar cover is opt-in and Windows only', async () => {
  const windows = await harness()
  try {
    assert.equal(windows.input('显示歌曲封面').checked, false)
    await React.act(async () => windows.input('显示歌曲封面').click())
    const [, args] = windows.calls[0]
    assert.equal(args.options.taskbarCover, true)
  } finally {
    await windows.dispose()
  }

  const android = await harness({ platform: 'Android' })
  try {
    assert.equal(android.card('任务栏封面'), null)
    assert.equal(document.getElementById('personalization-taskbar-heading'), null)
  } finally {
    await android.dispose()
  }
})

test('taskbar cover reports persistence failures and remains disabled until the save settles', async () => {
  let reject
  const app = await harness({
    persist: () =>
      new Promise((resolve, fail) => {
        reject = fail
      })
  })
  try {
    await React.act(async () => app.input('显示歌曲封面').click())
    assert.equal(app.input('显示歌曲封面').checked, false)
    assert.equal(app.input('显示歌曲封面').disabled, true)
    await React.act(async () => reject(Error('无法保存任务栏设置')))
    assert.equal(app.input('显示歌曲封面').disabled, false)
    assert.match(document.querySelector('[role="alert"]').textContent, /无法保存任务栏设置/)
    assert.equal(app.diagnostics.length, 1)
    assert.equal(
      app.diagnostics[0].operation,
      'app/src/features/settings/appearance/taskbar-settings.tsx'
    )
  } finally {
    await app.dispose()
  }
})
