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
  try {
    await fn(root)
  } finally {
    await React.act(() => root.unmount())
    dom.window.close()
  }
}

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
    const musicTools = load('../src/features/tools/use-music-tools.ts', {
      '@/plugins/types': routing,
      '@/plugins/host': { usePlugins: () => ({ plugins: [tool], loaded }) }
    })
    const { ToolsPage } = load('../src/features/tools/tools-page.tsx', {
      react: React,
      'react/jsx-runtime': jsx,
      'lucide-react': { Wrench: 'svg', ArrowUpRight: 'svg' },
      '@/components/music/music-page': { MusicPage: 'section', MusicPageHeader: Header },
      '@/components/ui/button': { Button },
      './use-music-tools': musicTools,
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
