import { coverImage } from './helpers/cover-image.mjs'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { test } from 'node:test'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(path, modules) {
  modules = { '@/features/music/cover-image': coverImage, ...modules }
  const exports = {}
  runInNewContext(
    ts.transpileModule(readFileSync(new URL(path, import.meta.url), 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX }
    }).outputText,
    {
      exports,
      require: (name) => {
        assert.ok(name in modules, name)
        return modules[name]
      },
      console
    }
  )
  return exports
}
const Button = ({ variant: _variant, size: _size, ...props }) =>
  React.createElement('button', props)
const Switch = ({ checked, onCheckedChange, ...props }) =>
  React.createElement('button', {
    ...props,
    role: 'switch',
    'aria-checked': checked,
    onClick: () => onCheckedChange(!checked)
  })
const base = {
  '@/components/music/loading': { TextSkeleton: 'div' },
  react: React,
  'react/jsx-runtime': jsx,
  '@/components/music/music-page': { MusicPageHeader: 'header' },
  '@/components/ui/button': { Button },
  '@/components/ui/switch': { Switch }
}
async function domTest(work) {
  const dom = new JSDOM('<div id="root"></div>', { url: 'http://localhost/' })
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const root = createRoot(document.getElementById('root'))
  try {
    await work(dom, root)
  } finally {
    await React.act(async () => root.unmount())
    dom.window.close()
    delete globalThis.window
    delete globalThis.document
    delete globalThis.IS_REACT_ACT_ENVIRONMENT
  }
}
test('generated configuration autosaves on commit, retains failed drafts and retries', async () =>
  domTest(async (dom, root) => {
    let slider,
      fail = true
    const saved = []
    const field = load('../src/features/settings/plugins/configuration-field.tsx', {
      ...base,
      '@tauri-apps/plugin-dialog': { open: async () => null },
      '@/lib/player': { errorText: (e) => e.message },
      '@/components/ui/input': {
        Input: ({ onChange, ...props }) =>
          React.createElement('input', { ...props, onInput: onChange })
      },
      '@/components/ui/slider': {
        Slider: (props) => {
          slider = props
          return React.createElement('div', { 'data-slider': true })
        }
      },
      '@/components/ui/select': {
        Select: 'div',
        SelectContent: 'div',
        SelectItem: 'div',
        SelectTrigger: 'button',
        SelectValue: 'span'
      }
    }).ConfigurationField
    const definition = {
      key: 'label',
      title: '名称',
      description: '插件名称',
      schema: { type: 'string' },
      default: 'default',
      apply: 'live',
      editor: { kind: 'text', options: [] }
    }
    await React.act(async () =>
      root.render(
        React.createElement(field, {
          field: definition,
          value: 'old',
          disabled: false,
          save: async (key, value) => {
            saved.push([key, value])
            if (fail) throw new Error('磁盘不可写')
          },
          reset: async () => {}
        })
      )
    )
    const input = document.querySelector('input')
    await React.act(async () => {
      input.value = 'new'
      input.dispatchEvent(new dom.window.Event('input', { bubbles: true }))
    })
    assert.equal(saved.length, 0)
    await React.act(async () =>
      input.dispatchEvent(new dom.window.FocusEvent('focusout', { bubbles: true }))
    )
    assert.equal(saved.length, 1)
    assert.equal(input.value, 'new')
    assert.match(document.querySelector('[role=alert]').textContent, /磁盘不可写/)
    fail = false
    await React.act(async () =>
      [...document.querySelectorAll('button')].find((b) => b.textContent === '重试保存').click()
    )
    assert.equal(saved.length, 2)
    assert.equal(document.querySelector('[role=alert]'), null)
    await React.act(async () =>
      root.render(
        React.createElement(field, {
          field: {
            ...definition,
            key: 'duration',
            schema: { type: 'number' },
            default: 8,
            editor: { kind: 'slider', min: 1, max: 30, options: [] }
          },
          value: 8,
          disabled: false,
          save: async (key, value) => saved.push([key, value]),
          reset: async () => {}
        })
      )
    )
    await React.act(async () => slider.onValueChange(12))
    assert.equal(saved.length, 2)
    await React.act(async () => slider.onValueCommitted(12))
    assert.deepEqual(saved.at(-1), ['duration', 12])
  }))
test('settings page edits while disabled, handles conflicts and requires loaded custom pages', async () =>
  domTest(async (_dom, root) => {
    let settings = {
      definition: {
        version: 1,
        page: { mode: 'generated' },
        fields: [{ key: 'duration', title: '时间', editor: { kind: 'slider' } }]
      },
      snapshot: { values: { duration: 8 }, revision: 0, diagnostics: {}, pendingReload: false },
      grants: []
    }
    let fieldProps, customProps
    const calls = []
    let fail = false
    const Page = load('../src/features/settings/plugins/configuration-page.tsx', {
      ...base,
      'lucide-react': { ArrowLeft: 'svg', FolderOpen: 'svg', RotateCw: 'svg', Trash2: 'svg' },
      '@tauri-apps/api/core': { isTauri: () => false },
      '@tauri-apps/api/event': { listen: async () => () => {} },
      '@/plugins/types': { pluginPath: (id, path) => `/plugins/${id}${path}` },
      '@/plugins/host': {
        PluginPageHost: (props) => {
          customProps = props
          return React.createElement('p', {}, props.path)
        }
      },
      './configuration-field': {
        ConfigurationField: (props) => {
          fieldProps = props
          return React.createElement('div', {}, props.value)
        }
      },
      '@/lib/player': {
        errorText: (e) => e.message,
        nativeCall: async (command, args) => {
          calls.push([command, args])
          if (args.operation === 'update') {
            if (fail) {
              settings = { ...settings, snapshot: { ...settings.snapshot, revision: 4 } }
              throw new Error('修订冲突')
            }
            assert.equal(args.args.revision, settings.snapshot.revision)
            settings = {
              ...settings,
              snapshot: {
                ...settings.snapshot,
                values: { ...settings.snapshot.values, ...args.args.patch },
                revision: settings.snapshot.revision + 1,
                pendingReload: true
              }
            }
          }
          return settings
        }
      }
    }).PluginConfigurationPage
    const plugin = {
      manifest: {
        id: 'sample',
        name: '示例',
        permissions: ['config'],
        configuration: 'configuration.json',
        contributes: { pages: [{ id: 'settings', path: '/settings' }] }
      },
      loaded: false,
      generation: 1
    }
    await React.act(async () => root.render(React.createElement(Page, { plugin, back: () => {} })))
    assert.equal(fieldProps.disabled, false)
    await React.act(async () => fieldProps.save('duration', 12))
    assert.match(document.body.textContent, /重载插件后生效/)
    fail = true
    await React.act(async () => {
      await assert.rejects(fieldProps.save('duration', 14), /修订冲突/)
    })
    fail = false
    await React.act(async () => fieldProps.save('duration', 14))
    assert.equal(calls.filter(([, args]) => args.operation === 'update').at(-1)[1].args.revision, 4)
    settings = {
      ...settings,
      definition: { version: 1, page: { mode: 'custom', pageId: 'settings' }, fields: [] }
    }
    await React.act(async () => {
      root.unmount()
    })
    const second = createRoot(document.getElementById('root'))
    try {
      await React.act(async () =>
        second.render(React.createElement(Page, { plugin, back: () => {} }))
      )
      assert.match(document.body.textContent, /自定义配置页需要插件正常加载/)
      assert.equal(customProps, undefined)
      await React.act(async () =>
        second.render(
          React.createElement(Page, { plugin: { ...plugin, loaded: true }, back: () => {} })
        )
      )
      assert.equal(customProps.path, '/plugins/sample/settings')
      await React.act(async () => customProps.navigate('/settings/child'))
      assert.equal(customProps.path, '/plugins/sample/settings/child')
    } finally {
      await React.act(async () => second.unmount())
    }
  }))
test('SDK keeps configuration navigation local and rejects stale configuration requests', async () =>
  domTest(async (_dom, root) => {
    const scopeModule = load('../src/plugins/scope.tsx', { react: React })
    const routing = load('../src/plugins/types.ts', {})
    let sdkConfig, route, navigate, listener, finish
    let unlistened = 0
    const globalRoutes = [],
      localRoutes = []
    const sdk = load('../src/plugins/sdk.ts', {
      react: React,
      './collection-tabs': {},
      './scope': scopeModule,
      './types': routing,
      './song-components': {},
      '@/components/music/music-page': { MusicPageHeader: 'header' },
      '@/components/ui/button': { Button },
      '@/components/ui/progress': {},
      '@/components/ui/input': {},
      '@/components/animate-ui/components/base/tabs': {},
      './downloaded-song-list': {},
      './icon': {},
      '@tauri-apps/api/event': {
        listen: async (_event, callback) => {
          listener = callback
          return () => unlistened++
        }
      },
      '@/lib/player': {
        nativeCall: async () =>
          await new Promise((resolve) => {
            finish = resolve
          })
      },
      '@/features/settings/use-theme': {},
      '@/components/music/use-cover-source': {},
      '@/features/workspace/music-navigation': {
        useMusicNavigation: () => ({
          page: { view: 'library' },
          navigate: (...args) => globalRoutes.push(args)
        })
      }
    })
    const scope = {
      descriptor: { generation: 1, manifest: { id: 'sample', permissions: ['config', 'ui'] } },
      active: true,
      events: new Map(),
      listeners: new Set()
    }
    function Probe() {
      sdkConfig = sdk.usePluginConfig()
      route = sdk.usePluginRoute()
      navigate = sdk.usePluginNavigate()
      return null
    }
    await React.act(async () =>
      root.render(
        React.createElement(
          scopeModule.PluginScope.Provider,
          { value: scope },
          React.createElement(
            scopeModule.PluginNavigationScope.Provider,
            {
              value: {
                pathname: '/settings',
                search: '',
                navigate: (path) => localRoutes.push(path)
              }
            },
            React.createElement(Probe)
          )
        )
      )
    )
    assert.equal(route.pathname, '/settings')
    navigate('/settings/child')
    assert.deepEqual(localRoutes, ['/settings/child'])
    assert.equal(globalRoutes.length, 0)
    assert.throws(() => navigate('/../escape'))
    const cleanup = await sdkConfig.subscribe(() => assert.fail('stale result delivered'))
    listener({ payload: { pluginId: 'other' } })
    assert.equal(finish, undefined)
    const pending = sdkConfig.getSnapshot()
    scope.active = false
    finish('{}')
    await assert.rejects(pending, /禁用/)
    await assert.rejects(sdkConfig.getSnapshot(), /禁用/)
    cleanup()
    assert.equal(unlistened, 1)
    assert.equal(scope.cleanups.size, 0)
  }))
