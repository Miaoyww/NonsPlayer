import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from './helpers/diagnostic-vm.mjs'
import * as React from 'react'
import * as jsx from 'react/jsx-runtime'
import { createRoot } from 'react-dom/client'
import { JSDOM } from 'jsdom'
import ts from 'typescript'

function load(path, modules) {
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
      }
    }
  )
  return exports
}
const Button = ({ variant: _, size: __, ...props }) => React.createElement('button', props)
const Header = ({ title }) => React.createElement('h1', null, title)
async function renderTest(fn) {
  const dom = new JSDOM('<div id="root"></div>')
  globalThis.window = dom.window
  globalThis.document = dom.window.document
  globalThis.IS_REACT_ACT_ENVIRONMENT = true
  const root = createRoot(document.getElementById('root'))
  const click = (label) =>
    React.act(async () => {
      const button = [...document.querySelectorAll('button')].find(
        (button) => button.textContent === label
      )
      assert.ok(button, label)
      button.click()
    })
  try {
    await fn(root, click)
  } finally {
    await React.act(() => root.unmount())
    dom.window.close()
  }
}

test('converter deduplicates selection, continues after errors and skips completed files on retry', async () => {
  await renderTest(async (root, click) => {
    const selected = [
      { root: '*', path: 'C:/music/bad.ncm', name: 'bad.ncm' },
      { root: '*', path: 'C:/music/good.ncm', name: 'good.ncm' }
    ]
    const calls = []
    let fail = true
    const files = {
      pickAudio: async () => selected,
      decodeAudio: async (root, path) => {
        calls.push([root, path])
        if (path.includes('bad') && fail) throw new Error('目标已存在')
        return { path: path.replace('.ncm', '.flac'), format: 'flac', bytes: 42 }
      }
    }
    const { ConverterPage } = load('../../plugins/ncm-converter/frontend/index.tsx', {
      react: React,
      'react/jsx-runtime': jsx,
      '@app/plugin-sdk': {
        Button,
        MusicPageHeader: Header,
        Progress: () => null,
        usePluginFiles: () => files
      }
    })
    await React.act(() => root.render(React.createElement(ConverterPage)))
    await click('选择 NCM 文件')
    await click('选择 NCM 文件')
    assert.equal(document.querySelectorAll('li').length, 2)
    await click('开始转换')
    assert.equal(calls.length, 2)
    assert.match(document.body.textContent, /目标已存在/)
    assert.match(document.body.textContent, /good.flac/)
    fail = false
    await click('开始转换')
    assert.equal(calls.length, 3)
    assert.match(document.body.textContent, /已完成 2 个/)
    assert.ok(
      [...document.querySelectorAll('button')].find((b) => b.textContent === '开始转换').disabled
    )
  })
})

test('tools page displays active tool contributions and preserves plugin page navigation', async () => {
  await renderTest(async (root) => {
    const tool = {
      manifest: {
        id: 'convert',
        name: '转换',
        description: '处理音乐',
        contributes: {
          pages: [{ id: 'main', path: '/' }],
          navigation: [{ id: 'entry', label: '格式转换', page: 'main', category: 'tool' }]
        }
      }
    }
    let loaded = new Map([['convert', {}]])
    const navigation = []
    const routing = load('../src/plugins/types.ts', {})
    const { ToolsPage } = load('../src/features/tools/tools-page.tsx', {
      react: React,
      'react/jsx-runtime': jsx,
      'lucide-react': { Wrench: 'svg', ArrowUpRight: 'svg' },
      '@/components/music/music-page': { MusicPage: 'section', MusicPageHeader: Header },
      '@/components/ui/button': { Button },
      '@/plugins/types': routing,
      '@/plugins/host': { usePlugins: () => ({ plugins: [tool], loaded }) },
      '@/features/workspace/music-navigation': {
        useMusicNavigation: () => ({ navigate: (...args) => navigation.push(args) })
      }
    })
    await React.act(() => root.render(React.createElement(ToolsPage)))
    await React.act(() => document.querySelector('button').click())
    assert.deepEqual(navigation, [['plugin', '/plugins/convert/']])
    loaded = new Map()
    await React.act(() => root.render(React.createElement(ToolsPage)))
    assert.match(document.body.textContent, /还没有启用音乐工具/)
    assert.equal(document.querySelectorAll('button').length, 0)
  })
})
