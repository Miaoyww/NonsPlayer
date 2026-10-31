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

const compiled = ts.transpileModule(
  readFileSync(new URL('../src/features/settings/pages/playback.tsx', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
).outputText

async function harness({
  platform = 'Windows',
  supportsDevices = true,
  deviceId = null,
  deviceList = [],
  chooseAudioOutput,
  persist = async () => {}
} = {}) {
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
    deviceId,
    playbackOptions: { fades: false, crossfade: false, crossfadeDurationMs: 3000 }
  }
  let devices = deviceList
  const listeners = new Set()
  const calls = []
  const browserCalls = []
  const required = []
  const modules = {
    react: React,
    'react/jsx-runtime': jsx,
    '@tauri-apps/api/core': { isTauri: () => true },
    '@/lib/platform': { isAndroid: () => platform === 'Android' },
    '@/components/error-notice': errorNoticeModule,
    '@/components/ui/button': {
      Button: ({ variant: _variant, size: _size, ...props }) => React.createElement('button', props)
    },
    '@/components/ui/switch': {
      Switch: ({ onCheckedChange, ...props }) =>
        React.createElement('input', {
          ...props,
          type: 'checkbox',
          onChange: (event) => onCheckedChange(event.target.checked)
        })
    },
    '@/components/ui/select': {
      Select: ({ value, items, disabled, onOpenChange, onValueChange, children }) =>
        React.createElement(
          'select',
          {
            value: value ?? '',
            disabled,
            onFocus: () => onOpenChange?.(true),
            onChange: (event) => {
              const selected = event.target.value
              onValueChange(
                items
                  ? items.find((item) => String(item.value ?? '') === selected).value
                  : Number(selected)
              )
            }
          },
          items
            ? items.map((item) =>
                React.createElement(
                  'option',
                  { key: item.value ?? '', value: item.value ?? '' },
                  item.label
                )
              )
            : children
        ),
      SelectContent: ({ children }) => children,
      SelectGroup: ({ children }) => children,
      SelectItem: ({ value, children }) => React.createElement('option', { value }, children),
      SelectTrigger: () => null,
      SelectValue: () => null
    },
    '@/features/settings/settings-card': {
      SettingsCard: ({ title, description, children }) =>
        React.createElement(
          'section',
          { 'data-title': title },
          React.createElement('p', {}, description),
          children
        )
    },
    '@/features/playback/music-options': {
      useMusicOptions: () => ({ options: { allowDowngrade: true }, busy: false, update() {} })
    },
    '@/features/playback/audio-output': {
      browserOutput: () => {
        browserCalls.push(['browserOutput'])
        return { supportsDevices }
      },
      browserOutputDevices: async () => {
        browserCalls.push(['browserOutputDevices'])
        return devices
      },
      selectBrowserOutput: async (id) => {
        browserCalls.push(['selectBrowserOutput', id])
      }
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
        if (command === 'player_devices') return devices
        assert.ok(['player_device', 'set_playback_options'].includes(command), command)
        await persist(command, args)
        state = {
          ...state,
          ...(command === 'player_device'
            ? { deviceId: args.deviceId }
            : { playbackOptions: args.options })
        }
        listeners.forEach((listener) => listener())
      }
    }
  }
  const exports = {}
  runInNewContext(compiled, {
    exports,
    navigator: { userAgent: platform, mediaDevices: { selectAudioOutput: chooseAudioOutput } },
    require(name) {
      required.push(name)
      assert.ok(Object.hasOwn(modules, name), `unexpected dependency: ${name}`)
      return modules[name]
    }
  })
  const root = createRoot(document.getElementById('root'))
  const card = (title) => document.querySelector(`[data-title="${title}"]`)
  const select = () => card('输出设备').querySelector('select')
  const change = async (element, value) => {
    await React.act(async () => {
      element.value = value
      element.dispatchEvent(new dom.window.Event('change', { bubbles: true }))
    })
  }
  await React.act(async () => root.render(React.createElement(exports.PlaybackPage)))
  return {
    card,
    select,
    change,
    calls,
    browserCalls,
    required,
    state: () => state,
    setDevices: (value) => {
      devices = value
    },
    input: (label) =>
      [...document.querySelectorAll('label')]
        .find((element) => element.textContent.includes(label))
        .querySelector('input'),
    async refresh() {
      await React.act(async () => select().focus())
    },
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

test('playback fades and crossfade wait for persistence and retain the previous setting on failure', async () => {
  let pending
  const app = await harness({
    persist: () =>
      new Promise((resolve, reject) => {
        pending = { resolve, reject }
      })
  })
  try {
    assert.equal(app.input('启用播放/暂停渐变').checked, false)
    assert.equal(app.input('启用交叉淡入淡出').checked, false)
    await React.act(async () => app.input('启用播放/暂停渐变').click())
    assert.equal(app.input('启用播放/暂停渐变').checked, false)
    assert.equal(app.input('启用交叉淡入淡出').disabled, true)
    assert.match(document.querySelector('[role="status"]').textContent, /正在保存/)
    await React.act(async () => pending.resolve())
    assert.equal(app.input('启用播放/暂停渐变').checked, true)
    await React.act(async () => app.input('启用交叉淡入淡出').click())
    await React.act(async () => pending.reject(Error('无法保存设置')))
    assert.equal(app.input('启用交叉淡入淡出').checked, false)
    assert.match(document.querySelector('[role="alert"]').textContent, /无法保存设置/)
    await React.act(async () => app.input('启用交叉淡入淡出').click())
    await React.act(async () => pending.resolve())
    assert.equal(app.input('启用交叉淡入淡出').checked, true)
    assert.equal(app.input('启用播放/暂停渐变').checked, true)
    const duration = app.card('交叉淡入淡出').querySelector('select')
    await app.change(duration, '6000')
    assert.equal(duration.value, '3000')
    await React.act(async () => pending.resolve())
    assert.equal(duration.value, '6000')
    assert.deepEqual(JSON.parse(JSON.stringify(app.calls.at(-1))), [
      'set_playback_options',
      { options: { fades: true, crossfade: true, crossfadeDurationMs: 6000 } }
    ])
  } finally {
    await app.dispose()
  }
})

for (const platform of ['Windows', 'Macintosh', 'Linux'])
  test(`native output devices use opaque IDs and preserve transitions on ${platform}`, async () => {
    const devices = [
      { id: '{0.0.0.00000000}.{6f6b2f86-01}', name: 'USB Audio' },
      { id: '{0.0.0.00000000}.{6f6b2f86-02}', name: 'USB Audio' }
    ]
    let pending
    let chooserCalls = 0
    const app = await harness({
      platform,
      deviceId: devices[0].id,
      deviceList: devices,
      chooseAudioOutput: async () => {
        chooserCalls++
        return { deviceId: 'browser-only-id' }
      },
      persist: () =>
        new Promise((resolve, reject) => {
          pending = { resolve, reject }
        })
    })
    try {
      assert.equal(app.card('播放引擎'), null)
      assert.equal(app.card('无缝播放（Gapless）'), null)
      assert.equal(app.select().disabled, false)
      assert.equal(app.input('启用播放/暂停渐变').disabled, false)
      assert.equal(app.input('启用交叉淡入淡出').disabled, false)
      assert.deepEqual(app.calls, [['player_devices', undefined]])
      assert.deepEqual(
        [...app.select().options].map((option) => [option.value, option.textContent]),
        [['', '跟随系统默认'], ...devices.map(({ id, name }) => [id, name])]
      )
      assert.equal(
        Boolean(app.card('输出设备').querySelector('button')),
        false,
        'native output must not offer the browser device chooser'
      )
      await app.change(app.select(), devices[1].id)
      assert.equal(app.select().disabled, true)
      assert.equal(app.state().deviceId, devices[0].id)
      assert.equal(app.calls.at(-1)[0], 'player_device')
      assert.equal(app.calls.at(-1)[1].deviceId, devices[1].id)
      await React.act(async () => pending.resolve())
      assert.equal(app.select().value, devices[1].id)
      await app.change(app.select(), '')
      assert.equal(app.calls.at(-1)[1].deviceId, null)
      await React.act(async () => pending.reject(Error('设备切换失败')))
      assert.equal(app.select().value, devices[1].id)
      assert.equal(app.select().disabled, false)
      assert.match(document.querySelector('[role="alert"]').textContent, /设备切换失败/)
      await app.change(app.select(), '')
      await React.act(async () => pending.resolve())
      assert.equal(app.state().deviceId, null)
      assert.equal(app.select().value, '')
      const connected = { id: 'native:hotplug:3', name: 'HDMI Audio' }
      app.setDevices([...devices, connected])
      await app.refresh()
      assert.equal(app.calls.at(-1)[0], 'player_devices')
      assert.equal(app.select().options.length, 4)
      assert.equal(app.select().options[3].value, connected.id)
      assert.equal(chooserCalls, 0)
      assert.deepEqual(app.browserCalls, [])
      assert.equal(app.required.includes('@/features/playback/audio-output'), false)
    } finally {
      await app.dispose()
    }
  })

test('Android retains browser device selection, refresh and rollback on persistence failure', async () => {
  const devices = [
    { id: 'browser-saved', name: 'Headphones' },
    { id: 'browser-next', name: 'Speakers' }
  ]
  let fail = true
  const app = await harness({
    platform: 'Android',
    deviceId: devices[0].id,
    deviceList: devices,
    persist: async () => {
      if (fail) throw Error('无法保存输出设备')
    }
  })
  try {
    assert.deepEqual(app.calls, [])
    assert.equal(app.select().value, devices[0].id)
    assert.equal(app.select().disabled, false)
    assert.equal(app.input('启用播放/暂停渐变').disabled, false)
    assert.equal(app.input('启用交叉淡入淡出').disabled, false)
    await app.change(app.select(), devices[1].id)
    assert.deepEqual(app.browserCalls, [
      ['browserOutput'],
      ['browserOutputDevices'],
      ['selectBrowserOutput', devices[1].id],
      ['selectBrowserOutput', devices[0].id]
    ])
    assert.equal(app.calls.at(-1)[1].deviceId, devices[1].id)
    assert.equal(app.select().value, devices[0].id)
    assert.equal(app.select().disabled, false)
    assert.match(document.querySelector('[role="alert"]').textContent, /无法保存输出设备/)
    fail = false
    await app.change(app.select(), devices[1].id)
    assert.equal(app.select().value, devices[1].id)
    await app.change(app.select(), '')
    assert.equal(app.browserCalls.at(-1)[1], null)
    assert.equal(app.calls.at(-1)[1].deviceId, null)
    assert.equal(app.state().deviceId, null)
    app.setDevices([...devices, { id: 'browser-hotplug', name: 'USB' }])
    await app.refresh()
    assert.equal(app.browserCalls.at(-1)[0], 'browserOutputDevices')
    assert.equal(app.select().options[3].value, 'browser-hotplug')
    assert.equal(
      app.calls.some(([command]) => command === 'player_devices'),
      false
    )
  } finally {
    await app.dispose()
  }
})

test('Android requests browser output permission from the chooser click', async () => {
  let chooserCalls = 0
  let app
  app = await harness({
    platform: 'Android',
    chooseAudioOutput: async () => {
      chooserCalls++
      app.setDevices([{ id: 'browser-permitted', name: 'Permitted output' }])
      return { deviceId: 'browser-permitted' }
    }
  })
  try {
    const button = app.card('输出设备').querySelector('button')
    assert.ok(button)
    assert.equal(chooserCalls, 0)
    await React.act(async () => button.click())
    assert.equal(chooserCalls, 1)
    assert.equal(app.state().deviceId, 'browser-permitted')
    assert.equal(app.select().value, 'browser-permitted')
    assert.deepEqual(app.browserCalls.slice(-2), [
      ['selectBrowserOutput', 'browser-permitted'],
      ['browserOutputDevices']
    ])
    assert.equal(app.calls.at(-1)[0], 'player_device')
    assert.equal(app.calls.at(-1)[1].deviceId, 'browser-permitted')
  } finally {
    await app.dispose()
  }
})

test('Android without browser device support disables only output selection', async () => {
  const app = await harness({ platform: 'Android', supportsDevices: false })
  try {
    assert.equal(app.select().disabled, true)
    assert.match(app.card('输出设备').textContent, /不支持输出设备选择/)
    assert.equal(app.input('启用播放/暂停渐变').disabled, false)
    assert.equal(app.input('启用交叉淡入淡出').disabled, false)
    assert.deepEqual(app.calls, [])
  } finally {
    await app.dispose()
  }
})
