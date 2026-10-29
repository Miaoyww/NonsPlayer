import { cp, readFile, readdir, rename, writeFile } from 'node:fs/promises'
import { basename, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { formatErrors, validateManifest } from './manifest.mjs'
import { validId } from './util.mjs'

const template = fileURLToPath(new URL('../template', import.meta.url))

function pluginId(directory) {
  return basename(directory)
    .toLowerCase()
    .replace(/[^a-z0-9-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .replace(/^[^a-z]+/, '')
}

export async function createPlugin(directory, options = {}) {
  const target = resolve(directory)
  const id = options.id ?? pluginId(target)
  if (!validId(id))
    throw new Error(`插件 id 无效：${id}，请使用 --id 指定 1 至 64 位小写字母、数字或连字符`)
  const entries = await readdir(target).catch(() => [])
  if (entries.length) throw new Error(`目标目录不为空：${target}`)
  await cp(template, target, { recursive: true })
  // npm omits .gitignore files from tarballs; the template ships a plain name.
  await rename(join(target, 'gitignore'), join(target, '.gitignore')).catch(() => {})

  const manifestPath = join(target, 'manifest.json')
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
  manifest.id = id
  manifest.name = options.name ?? id
  if (options.description) manifest.description = options.description
  const errors = await validateManifest(manifest)
  if (errors.length) throw new Error(`模板 manifest 未通过校验：\n${formatErrors(errors)}`)
  await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)

  const packagePath = join(target, 'package.json')
  const pkg = JSON.parse(await readFile(packagePath, 'utf8'))
  pkg.name = `nons-plugin-${id}`
  pkg.version = manifest.version
  await writeFile(packagePath, `${JSON.stringify(pkg, null, 2)}\n`)
  return { target, id }
}
