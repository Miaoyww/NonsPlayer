import { readFile, stat } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import Ajv2020 from 'ajv/dist/2020.js'
import semver from 'semver'
import { checkedFile, safeRelative, validHost, validId } from './util.mjs'

const packageJson = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'))
export const SDK_VERSION = packageJson.version
export const PLUGIN_API_VERSION = packageJson.nons?.pluginApi ?? packageJson.version
export const UI_API_VERSION = packageJson.nons?.uiApi ?? packageJson.version

let schema
try {
  schema = JSON.parse(
    await readFile(new URL('../schemas/manifest.schema.json', import.meta.url), 'utf8')
  )
} catch {
  throw new Error('缺少 schemas/manifest.schema.json，请先在 SDK 仓库运行 pnpm run sync:assets')
}
const ajv = new Ajv2020({ allErrors: true, strict: false, validateFormats: false })
const validateSchema = ajv.compile(schema)

const EXPORT_PATTERN = /^[a-zA-Z0-9_]{1,128}$/

export function formatErrors(errors) {
  return errors.map((error) => `- ${error}`).join('\n')
}

export function validateStructure(manifest) {
  if (!validateSchema(manifest)) {
    return (validateSchema.errors ?? []).map(
      (error) => `${error.instancePath || '/'} ${error.message}`
    )
  }
  return validateSemantics(manifest)
}

function validateSemantics(manifest) {
  const errors = []
  const c = manifest.contributes ?? {}
  if (!validId(manifest.id)) errors.push('id 必须是 1 至 64 位小写字母、数字或连字符，且以字母开头')
  if (manifest.name.trim().length === 0 || Buffer.byteLength(manifest.name) > 128)
    errors.push('name 必须是 1 至 128 字节的非空文本')
  if (
    manifest.description != null &&
    (manifest.description.trim().length === 0 ||
      [...manifest.description].length > 1024 ||
      !/\S/.test(manifest.description))
  )
    errors.push('description 必须是 1 至 1024 个字符的非空文本')
  if (manifest.repository != null && !validRepository(manifest.repository))
    errors.push('repository 必须是不含凭证的 HTTP(S) 链接，最多 2048 个字符')
  if (!semver.valid(manifest.version, { loose: false }))
    errors.push('version 必须是 SemVer，例如 1.0.0')
  const engines = manifest.engines ?? {}
  for (const [key, current] of [
    ['pluginApi', PLUGIN_API_VERSION],
    ['uiApi', UI_API_VERSION]
  ]) {
    const range = engines[key]
    if (typeof range !== 'string' || !semver.validRange(range))
      errors.push(`engines.${key} 必须是有效的 SemVer 范围`)
    else if (!semver.satisfies(current, range))
      errors.push(`engines.${key} 与当前 SDK ${current} 不兼容`)
  }
  if (typeof engines.app !== 'string' || !semver.validRange(engines.app))
    errors.push('engines.app 必须是有效的 SemVer 范围')
  if (manifest.fileRoots?.length) {
    if (manifest.fileRoots.length !== 1 || manifest.fileRoots[0] !== '*')
      errors.push('fileRoots 当前仅支持 ["*"]')
    if (!manifest.permissions.includes('files:selected'))
      errors.push('fileRoots 需要 files:selected 权限')
  }
  for (const [entry, extension] of [
    [manifest.backend, '.wasm'],
    [manifest.frontend, '.mjs']
  ]) {
    if (entry == null) continue
    if (!safeRelative(entry))
      errors.push(`${extension === '.wasm' ? 'backend' : 'frontend'} 路径无效`)
    else if (!entry.endsWith(extension))
      errors.push(`${extension === '.wasm' ? 'backend' : 'frontend'} 必须以 ${extension} 结尾`)
  }
  if (manifest.configuration != null) {
    if (!safeRelative(manifest.configuration) || !manifest.configuration.endsWith('.json'))
      errors.push('configuration 必须是包内相对 JSON 路径')
  }
  const httpPermissions = manifest.permissions.some((permission) =>
    ['http:request', 'http:transfer'].includes(permission)
  )
  const hosts = manifest.httpHosts ?? []
  if (httpPermissions !== hosts.length > 0)
    errors.push('http:request/http:transfer 权限与 httpHosts 必须同时声明')
  if (hosts.length > 32 || new Set(hosts).size !== hosts.length)
    errors.push('httpHosts 最多 32 个且不能重复')
  for (const host of hosts)
    if (host !== '*' && !validHost(host)) errors.push(`httpHosts 包含无效域名：${host}`)

  const hasUiContributions =
    c.views?.length ||
    c.pages?.length ||
    c.navigation?.length ||
    c.contextMenus?.length ||
    c.windows?.length
  if (hasUiContributions && (manifest.frontend == null || !manifest.permissions.includes('ui')))
    errors.push('UI 扩展需要 frontend 与 ui 权限')
  if ((c.windows?.length ?? 0) > 4) errors.push('contributes.windows 最多 4 个')
  if (c.windows?.length && !manifest.permissions.includes('window:manage'))
    errors.push('contributes.windows 需要 window:manage 权限')
  if ((c.contextMenus?.length ?? 0) > 16) errors.push('contributes.contextMenus 最多 16 个')

  const ids = new Set()
  for (const [kind, items] of [
    ['views', c.views ?? []],
    ['pages', c.pages ?? []],
    ['windows', c.windows ?? []],
    ['contextMenus', c.contextMenus ?? []]
  ]) {
    for (const item of items) {
      if (!validId(item.id) || ids.has(item.id))
        errors.push(`contributes.${kind} 的 id 无效或重复：${item.id}`)
      else ids.add(item.id)
      if (!EXPORT_PATTERN.test(item.export))
        errors.push(`contributes.${kind} 的 export 无效：${item.export}`)
    }
  }
  if (c.views?.some((view) => view.slot !== 'main.overlay'))
    errors.push('contributes.views 仅支持 main.overlay')
  const paths = new Set()
  for (const page of c.pages ?? []) {
    if (
      !page.path.startsWith('/') ||
      /[?#\\%:]/.test(page.path) ||
      page.path.split('/').some((part) => part === '..' || part === '.') ||
      paths.has(page.path)
    )
      errors.push(`contributes.pages 路径无效或重复：${page.path}`)
    else paths.add(page.path)
  }
  for (const menu of c.contextMenus ?? []) {
    if (
      menu.target !== 'song' ||
      !validId(menu.source) ||
      menu.label.trim().length === 0 ||
      Buffer.byteLength(menu.label) > 128 ||
      (menu.icon != null && !validId(menu.icon))
    )
      errors.push(`contributes.contextMenus 无效：${menu.id}`)
  }
  for (const navigation of c.navigation ?? []) {
    if (
      (navigation.category != null && navigation.category !== 'tool') ||
      !validId(navigation.id) ||
      ids.has(navigation.id) ||
      navigation.label.trim().length === 0 ||
      Buffer.byteLength(navigation.label) > 128 ||
      !(c.pages ?? []).some((page) => page.id === navigation.page)
    )
      errors.push(`contributes.navigation 无效：${navigation.id}`)
    else ids.add(navigation.id)
  }
  return errors
}

function validRepository(repository) {
  if (
    [...repository].length > 2048 ||
    /\s/.test(repository) ||
    /[\u0000-\u001f\u007f]/.test(repository)
  )
    return false
  let url
  try {
    url = new URL(repository)
  } catch {
    return false
  }
  return (
    /^https?:\/\/\S+$/.test(repository) &&
    ['http:', 'https:'].includes(url.protocol) &&
    url.hostname.length > 0 &&
    url.username.length === 0 &&
    url.password.length === 0
  )
}

/** Structural checks plus packaged-file checks when root is the built plugin directory. */
export async function validateManifest(manifest, { root } = {}) {
  const errors = validateStructure(manifest)
  if (errors.length || !root) return errors
  const manifestFile = await checkedFile(root, 'manifest.json').catch((error) => {
    errors.push(error.message)
  })
  if (manifestFile && (await stat(manifestFile)).size > 64 * 1024)
    errors.push('manifest.json 超过 64KiB')
  if (errors.length) return errors
  for (const [entry, limit, label] of [
    [manifest.backend, 16 * 1024 * 1024, 'backend'],
    [manifest.frontend, 8 * 1024 * 1024, 'frontend']
  ]) {
    if (entry == null) continue
    try {
      const file = await checkedFile(root, entry)
      if ((await stat(file)).size > limit)
        errors.push(`${label} 超过 ${Math.round(limit / 1024 / 1024)}MiB 上限`)
    } catch (error) {
      errors.push(error.message)
    }
  }
  if (manifest.configuration != null) {
    try {
      await checkedFile(root, manifest.configuration)
    } catch (error) {
      errors.push(error.message)
    }
  }
  return errors
}
