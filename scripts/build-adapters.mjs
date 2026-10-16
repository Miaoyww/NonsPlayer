import { spawnSync } from 'node:child_process'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..')
for (const adapter of ['qqmusic', 'gdstudio', 'jbsou']) {
  const result = spawnSync(
    process.execPath,
    [resolve(root, 'adapters', adapter, 'scripts/build.mjs')],
    { cwd: root, stdio: 'inherit' }
  )
  if (result.error) throw result.error
  if (result.status !== 0) process.exit(result.status ?? 1)
}
