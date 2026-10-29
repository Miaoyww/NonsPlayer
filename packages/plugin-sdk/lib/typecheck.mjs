import { createRequire } from 'node:module'
import { join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import ts from 'typescript'

const packageRoot = fileURLToPath(new URL('..', import.meta.url))

function reactTypes() {
  for (const anchor of [
    pathToFileURL(join(packageRoot, 'package.json')),
    pathToFileURL(join(process.cwd(), 'package.json'))
  ]) {
    try {
      const require = createRequire(anchor)
      const directory = require.resolve('@types/react/package.json').replace(/package\.json$/, '')
      return { index: join(directory, 'index.d.ts'), wildcard: join(directory, '*') }
    } catch {}
  }
  return null
}

export function typecheck(project) {
  const configPath = join(project, 'tsconfig.json')
  const read = ts.readConfigFile(configPath, ts.sys.readFile)
  if (read.error) return { ok: false, text: format([read.error], project) }
  const types = reactTypes()
  const paths = {
    ...(read.config.compilerOptions?.paths ?? {}),
    '@nonsplayer/plugin-sdk': [join(packageRoot, 'index.d.ts')],
    '@app/plugin-sdk': [join(packageRoot, 'index.d.ts')]
  }
  if (types) {
    paths.react = [types.index]
    paths['react/*'] = [types.wildcard]
  }
  const parsed = ts.parseJsonConfigFileContent(read.config, ts.sys, project, { paths }, configPath)
  const program = ts.createProgram(parsed.fileNames, parsed.options)
  const diagnostics = [...parsed.errors, ...ts.getPreEmitDiagnostics(program)].filter(
    (diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error
  )
  return { ok: diagnostics.length === 0, text: format(diagnostics, project) }
}

function format(diagnostics, project) {
  return ts.formatDiagnostics(diagnostics, {
    getCanonicalFileName: (file) => file,
    getCurrentDirectory: () => project,
    getNewLine: () => '\n'
  })
}
