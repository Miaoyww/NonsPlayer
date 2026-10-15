import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'

export const sourceCapabilities = {}
runInNewContext(
  ts.transpileModule(
    readFileSync(
      new URL('../../src/features/music/source-capabilities.ts', import.meta.url),
      'utf8'
    ),
    {
      compilerOptions: { module: ts.ModuleKind.CommonJS }
    }
  ).outputText,
  { exports: sourceCapabilities }
)
