import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

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
    { exports, require: (name) => modules[name] }
  )
  return exports
}
