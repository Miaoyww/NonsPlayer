import { readFileSync } from 'node:fs'
import { runInThisContext } from 'node:vm'
import { LRUCache } from 'lru-cache'
import ts from 'typescript'

const code = ts.transpileModule(
  readFileSync(new URL('../../src/lib/runtime-cache.ts', import.meta.url), 'utf8'),
  { compilerOptions: { module: ts.ModuleKind.CommonJS } }
).outputText
export function createRuntimeCache() {
  const exports = {}
  runInThisContext(`(function(exports, require) { ${code}\n })`)(exports, (name) =>
    name === 'lru-cache' ? { LRUCache } : { reportCaughtError() {} }
  )
  return exports
}
