import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawn } from 'node:child_process'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const app = join(root, 'app')
const env = { ...process.env }
const args = process.argv.slice(2)

const require = createRequire(join(app, 'package.json'))
const cli = require.resolve('@tauri-apps/cli/tauri.js')
const child = spawn(process.execPath, [cli, ...args], {
  cwd: app,
  env,
  stdio: 'inherit'
})
child.on('error', (error) => {
  console.error(error.message)
  process.exitCode = 1
})
child.on('exit', (code, signal) => {
  process.exitCode = code ?? (signal ? 1 : 0)
})
