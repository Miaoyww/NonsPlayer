import { copyFile, mkdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

// The host repository owns the WIT contract and manifest schema; the package ships copies.
const packageRoot = fileURLToPath(new URL('..', import.meta.url))
const repositoryRoot = fileURLToPath(new URL('../../..', import.meta.url))
const assets = [
  ['plugins/wit/plugin.wit', 'wit/plugin.wit'],
  ['plugins/manifest.schema.json', 'schemas/manifest.schema.json']
]
for (const [source, target] of assets) {
  const destination = join(packageRoot, target)
  await mkdir(dirname(destination), { recursive: true })
  await copyFile(join(repositoryRoot, source), destination)
}
