import { lstat, realpath, stat } from 'node:fs/promises'
import { join, sep } from 'node:path'

const reserved = new Set([
  'con',
  'prn',
  'aux',
  'nul',
  ...Array.from({ length: 9 }, (_, index) => `com${index + 1}`),
  ...Array.from({ length: 9 }, (_, index) => `lpt${index + 1}`)
])

export function validId(id) {
  return id.length > 0 && id.length <= 64 && /^[a-z][a-z0-9-]{0,63}$/.test(id) && !reserved.has(id)
}

/** Mirrors the host rule for portable, relative manifest paths. */
export function safeRelative(path) {
  return (
    path.length > 0 &&
    path.length <= 512 &&
    !/[\\:\0%?#]/.test(path) &&
    path
      .split('/')
      .every((part) => part.length > 0 && part !== '.' && part !== '..' && !/[. ]$/.test(part))
  )
}

export function validHost(host) {
  if (host.length > 253 || !host.includes('.')) return false
  if (!host.split('.').every((label) => /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/.test(label)))
    return false
  // The host rejects IP literals; WHATWG URL parsing accepts dotted numeric hosts as IPv4.
  return !host.split('.').every((label) => /^\d+$/.test(label))
}

/** Rejects missing files, symlinks and paths escaping the plugin root. */
export async function checkedFile(root, relative) {
  if (!safeRelative(relative)) throw new Error(`资源路径无效：${relative}`)
  const rootReal = await realpath(root)
  let current = root
  for (const part of relative.split('/')) {
    current = join(current, part)
    const meta = await lstat(current).catch(() => {
      throw new Error(`文件不存在：${relative}`)
    })
    if (meta.isSymbolicLink()) throw new Error(`插件目录不允许链接：${relative}`)
  }
  const file = await realpath(current)
  if (!file.startsWith(rootReal + sep)) throw new Error(`资源越界：${relative}`)
  if (!(await stat(file)).isFile()) throw new Error(`不是普通文件：${relative}`)
  return file
}
