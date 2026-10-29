import { mkdir, readdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { zipSync } from 'fflate'

async function collect(root, relative = '') {
  const files = {}
  const entries = await readdir(join(root, relative), { withFileTypes: true })
  for (const entry of entries) {
    const path = relative ? `${relative}/${entry.name}` : entry.name
    if (entry.isSymbolicLink()) throw new Error(`插件资源不允许链接：${path}`)
    if (entry.isDirectory()) {
      Object.assign(files, await collect(root, path))
      continue
    }
    if (!entry.isFile()) throw new Error(`插件资源不是普通文件：${path}`)
    files[path] = new Uint8Array(await readFile(join(root, path)))
  }
  return files
}

export async function zipDirectory(root, destination) {
  const files = await collect(root)
  const archive = zipSync(files, { level: 6 })
  await mkdir(dirname(destination), { recursive: true })
  await writeFile(destination, archive)
}
