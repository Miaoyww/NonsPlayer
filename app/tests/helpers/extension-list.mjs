import { readFileSync } from 'node:fs'
import { runInNewContext } from './diagnostic-vm.mjs'
import ts from 'typescript'
import { Tabs } from '@base-ui/react/tabs'

export function loadExtensionList(modules) {
  const exports = {}
  runInNewContext(
    ts.transpileModule(
      readFileSync(
        new URL('../../src/features/settings/extension-list.tsx', import.meta.url),
        'utf8'
      ),
      { compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX } }
    ).outputText,
    {
      exports,
      require: (name) =>
        name === '@/components/animate-ui/components/base/tabs'
          ? { Tabs: Tabs.Root, TabsList: Tabs.List, TabsTab: Tabs.Tab, TabsPanel: Tabs.Panel }
          : modules[name]
    }
  )
  return exports
}
