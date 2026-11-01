#!/usr/bin/env node
// 生成发布头图，同步版本号与 CHANGELOG，然后提交、打标签并推送。
import { spawnSync } from 'node:child_process'
import { copyFile, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { createInterface } from 'node:readline/promises'
import { fileURLToPath, pathToFileURL } from 'node:url'
import process from 'node:process'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const PATHS = {
  packageJson: 'app/package.json',
  tauriConf: 'app/src-tauri/tauri.conf.json',
  cargoToml: 'app/src-tauri/Cargo.toml',
  cargoLock: 'app/src-tauri/Cargo.lock',
  changelog: 'CHANGELOG.md',
  headerSvg: 'docs/release-assets/header.svg',
  headerPng: 'docs/release-assets/header.png',
  editorHtml: 'docs/release-assets/editor.html'
}
const PROJECT_URL = 'https://raw.githubusercontent.com/Miaoyww/Nons'
const MAX_LENGTH = { version: 12, subtitle: 28, platform: 48, releaseKind: 18 }

const HELP = `用法：node scripts/release-version.mjs <版本号> [选项]

生成 1600 × 800 发布头图，同步版本文件与 CHANGELOG，提交后推送分支和标签。

版本号不带 v，例如 1.10.0。

选项：
  --subtitle <文字>       头图副标题，最长 28 字
  --platform <文字>       头图平台，最长 48 字
  --release-kind <文字>   头图发布类型，最长 18 字
  --intro <文字>          CHANGELOG 版本概述；省略时交互输入或使用建议文案
  --dry-run               只校验并生成临时预览，不修改仓库、不提交
  --yes, -y               跳过确认；缺失项使用当前头图或建议值
  --help, -h              显示本帮助

互动的缺失项会逐项提示；--yes 适合完整参数的非交互运行。
`

class ReleaseError extends Error {}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}

function escapeXml(value) {
  return value.replace(
    /[&<>]/g,
    (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[character]
  )
}

function escapeAttribute(value) {
  return escapeXml(value).replace(/"/g, '&quot;')
}

function elementText(source, tag, id) {
  const match = source.match(
    new RegExp(`<${tag}\\b[^>]*\\bid="${escapeRegExp(id)}"[^>]*>([\\s\\S]*?)</${tag}>`)
  )
  if (!match) throw new ReleaseError(`找不到 ${tag} #${id}`)
  return match[1].trim()
}

function replaceElementText(source, tag, id, value) {
  const pattern = new RegExp(
    `(<${tag}\\b[^>]*\\bid="${escapeRegExp(id)}"[^>]*>)([\\s\\S]*?)(</${tag}>)`
  )
  if (!pattern.test(source)) throw new ReleaseError(`找不到 ${tag} #${id}`)
  return source.replace(pattern, (_, open, inner, close) => {
    const leading = inner.match(/^\s*/)[0]
    const trailing = inner.match(/\s*$/)[0]
    return `${open}${leading}${escapeXml(value)}${trailing}${close}`
  })
}

function replaceInputValue(source, id, value) {
  const pattern = new RegExp(`(<input id="${escapeRegExp(id)}" value=")[^"]*(")`)
  if (!pattern.test(source)) throw new ReleaseError(`找不到输入框 #${id}`)
  return source.replace(pattern, (_, open, close) => `${open}${escapeAttribute(value)}${close}`)
}

async function readJsonFile(relativePath) {
  const path = join(ROOT, relativePath)
  return JSON.parse((await readFile(path, 'utf8')).replace(/^\uFEFF/, ''))
}

function jsonText(value) {
  return `${JSON.stringify(value, null, 2)}\n`
}

function updateJsonVersion(source, version) {
  const value = JSON.parse(source.replace(/^\uFEFF/, ''))
  value.version = version
  return `${source.startsWith('\uFEFF') ? '\uFEFF' : ''}${jsonText(value)}`
}

function updateCargoToml(source, version) {
  const marker = '[package]'
  const start = source.indexOf(marker)
  if (start < 0) throw new ReleaseError('Cargo.toml 缺少 [package]')
  const nextSection = source.indexOf('\n[', start + marker.length)
  const end = nextSection < 0 ? source.length : nextSection
  const section = source.slice(start, end)
  const updated = section.replace(/^version\s*=\s*"[^"]*"/m, `version = "${version}"`)
  if (updated === section) throw new ReleaseError('Cargo.toml 的 [package] 缺少 version')
  return source.slice(0, start) + updated + source.slice(end)
}

function updateCargoLock(source, version) {
  const pattern = /(\[\[package\]\]\r?\nname = "nons"\r?\nversion = ")[^"]+(")/
  if (!pattern.test(source)) throw new ReleaseError('Cargo.lock 中找不到 nons 包版本')
  return source.replace(pattern, `$1${version}$2`)
}

function parseVersion(value) {
  const match = /^(?:v)?((?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*))$/.exec(value.trim())
  if (!match) throw new ReleaseError(`版本号必须是 X.Y.Z，例如 1.10.0，收到：${value}`)
  return match[1]
}

function compareVersions(left, right) {
  const a = left.split('.').map(Number)
  const b = right.split('.').map(Number)
  for (let index = 0; index < 3; index++) {
    if (a[index] !== b[index]) return a[index] - b[index]
  }
  return 0
}

function today() {
  const now = new Date()
  return [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, '0'),
    String(now.getDate()).padStart(2, '0')
  ].join('-')
}

function checkText(name, label, value) {
  const text = value.trim()
  if (!text) throw new ReleaseError(`${label}不能为空`)
  if ([...text].length > MAX_LENGTH[name])
    throw new ReleaseError(`${label}超过 ${MAX_LENGTH[name]} 字：${text}`)
  return text
}

function suggestedIntro(version, subtitle, platform) {
  const parts = []
  if (/windows/i.test(platform)) parts.push('Windows 提供 MSI、EXE 安装包与免安装便携 ZIP')
  if (/linux/i.test(platform)) parts.push('Linux 提供 AppImage 与 deb')
  if (/android/i.test(platform)) parts.push('Android 提供 ARM64 试用 APK')
  const suffix = parts.length ? `：${parts.join('，')}。` : '。'
  return `NonsPlayer ${version} ${subtitle}。本次 Release 提供 **${platform}** 构建附件${suffix}`
}

function updateHeaderSvg(source, values) {
  let result = source
  result = replaceElementText(
    result,
    'title',
    'title',
    `NonsPlayer ${values.displayVersion} 发布头图`
  )
  const description = `蓝白扁平设计，${values.displayVersion}，${values.subtitle}，${values.platform}，${values.releaseKind}`
  result = replaceElementText(result, 'desc', 'desc', description)
  result = replaceElementText(result, 'text', 'version', values.displayVersion)
  result = replaceElementText(result, 'text', 'subtitle', values.subtitle)
  result = replaceElementText(result, 'text', 'platform', values.platform)
  result = replaceElementText(result, 'text', 'release-kind', values.releaseKind)
  return result
}

function updateEditorHtml(source, values) {
  let result = source
  result = replaceInputValue(result, 'edit-version', values.displayVersion)
  result = replaceInputValue(result, 'edit-subtitle', values.subtitle)
  result = replaceInputValue(result, 'edit-platform', values.platform)
  result = replaceInputValue(result, 'edit-release-kind', values.releaseKind)
  result = updateHeaderSvg(result, values)
  return result
}

function changelogEntry(values) {
  return {
    heading: `## [${values.version}] - ${values.date}`,
    image: `![NonsPlayer ${values.displayVersion} ${values.subtitle}头图](${PROJECT_URL}/${values.displayVersion}/docs/release-assets/header.png)`
  }
}

function updateChangelog(source, values) {
  const bom = source.startsWith('\uFEFF') ? '\uFEFF' : ''
  const normalized = (bom ? source.slice(1) : source).replace(/\r\n/g, '\n')
  const newline = source.includes('\r\n') ? '\r\n' : '\n'
  const lines = normalized.split('\n')
  const versionPattern = new RegExp(
    `^## \\[${escapeRegExp(values.version)}\\](?: - \\d{4}-\\d{2}-\\d{2})?\\s*$`
  )

  const existing = lines.findIndex((line) => versionPattern.test(line))
  if (existing >= 0) {
    const end = lines.findIndex((line, index) => index > existing && /^## /.test(line))
    const body = lines
      .slice(existing + 1, end < 0 ? undefined : end)
      .join('\n')
      .trim()
    if (!body) throw new ReleaseError(`CHANGELOG 中 [${values.version}] 条目为空`)
    return { text: source, inserted: false }
  }

  const unreleased = lines.findIndex((line) => /^## \[Unreleased\]\s*$/.test(line))
  if (unreleased < 0) throw new ReleaseError('CHANGELOG 缺少 ## [Unreleased]')
  const nextHeading = lines.findIndex((line, index) => index > unreleased && /^## /.test(line))
  const bodyEnd = nextHeading < 0 ? lines.length : nextHeading
  const body = lines.slice(unreleased + 1, bodyEnd)
  const firstContent = body.findIndex((line) => line.trim() !== '')
  const empty = firstContent < 0

  const { heading, image } = changelogEntry(values)
  const introLines = values.intro.trim().split('\n')
  const afterStart = empty ? bodyEnd : unreleased + 1 + firstContent
  const inserted = [
    '',
    heading,
    '',
    image,
    '',
    ...introLines,
    ...(afterStart < lines.length ? ['', ...lines.slice(afterStart)] : [''])
  ]
  const result = [...lines.slice(0, unreleased + 1), ...inserted].join(newline)
  return { text: `${bom}${result}`, inserted: true, empty }
}

async function renderHeaderPng(svg, output) {
  const appRequire = createRequire(join(ROOT, 'app/package.json'))
  let chromium
  try {
    ;({ chromium } = appRequire('playwright'))
  } catch {
    throw new ReleaseError('缺少 Playwright，请先运行 pnpm --dir app install')
  }
  let browser
  try {
    browser = await chromium.launch()
  } catch (error) {
    throw new ReleaseError(
      `无法启动 Chromium：${error.message}\n请运行 pnpm --dir app exec playwright install chromium`
    )
  }
  try {
    const page = await browser.newPage({
      viewport: { width: 1600, height: 800 },
      deviceScaleFactor: 1
    })
    await page.setContent(
      `<!doctype html><html><head><style>html,body{margin:0;padding:0;background:#F5F8FC}svg{display:block}</style></head><body>${svg}</body></html>`,
      { waitUntil: 'load' }
    )
    await page.evaluate(async () => {
      await document.fonts.ready
      await Promise.all(
        [...document.images].map((image) =>
          image.complete
            ? image.decode?.().catch(() => {})
            : new Promise((resolveImage) => {
                image.onload = resolveImage
                image.onerror = resolveImage
              })
        )
      )
    })
    await page.screenshot({
      path: output,
      type: 'png',
      clip: { x: 0, y: 0, width: 1600, height: 800 }
    })
  } finally {
    await browser.close()
  }

  const png = await readFile(output)
  if (png.readUInt32BE(16) !== 1600 || png.readUInt32BE(20) !== 800)
    throw new ReleaseError('生成的 PNG 不是 1600 × 800')
}

function run(command, args, options = {}) {
  return spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    ...options
  })
}

function git(args) {
  return run('git', args)
}

function checkedGit(args) {
  const result = git(args)
  if (result.error) throw new ReleaseError(`无法执行 git：${result.error.message}`)
  if (result.status !== 0)
    throw new ReleaseError(`git ${args.join(' ')} 失败：${(result.stderr || result.stdout).trim()}`)
  return result.stdout.trim()
}

function verifyPreconditions(tag, dryRun) {
  const branch = checkedGit(['branch', '--show-current'])
  if (!branch) throw new ReleaseError('当前不在分支上，无法推送')

  const status = checkedGit(['status', '--porcelain'])
  if (status) {
    if (!dryRun) throw new ReleaseError(`工作区不干净，请先提交或清理：\n${status}`)
    console.warn(`警告：工作区有未提交改动，预览不受影响。\n${status}\n`)
  }

  const localTag = git(['show-ref', '--verify', '--quiet', `refs/tags/${tag}`])
  if (localTag.status === 0) throw new ReleaseError(`标签 ${tag} 已存在`)

  const remote = git(['ls-remote', '--tags', 'origin', tag])
  if (
    remote.status === 0 &&
    new RegExp(`refs/tags/${escapeRegExp(tag)}(\\^\\{\\})?$`).test(remote.stdout)
  )
    throw new ReleaseError(`远程已存在标签 ${tag}`)
  if (remote.status !== 0) console.warn('警告：无法查询远程标签，推送时可能因标签冲突失败。')

  return branch
}

async function validateWithReleaseScript(tag) {
  const notes = join(tmpdir(), `nons-release-notes-${process.pid}.md`)
  const commands = process.platform === 'win32' ? ['python', 'python3'] : ['python3', 'python']
  let lastError
  try {
    for (const command of commands) {
      const result = run(command, ['scripts/release.py', 'notes', tag, notes])
      if (result.error?.code === 'ENOENT') {
        lastError = result.error
        continue
      }
      if (result.status !== 0)
        throw new ReleaseError(`发布元数据校验失败：\n${(result.stderr || result.stdout).trim()}`)
      return
    }
    throw new ReleaseError(`找不到 Python，无法校验发布元数据：${lastError?.message ?? ''}`)
  } finally {
    await rm(notes, { force: true })
  }
}

function formatFiles() {
  const args = [
    '--dir',
    'app',
    'exec',
    'prettier',
    '--write',
    '--ignore-unknown',
    '../CHANGELOG.md',
    '../docs/release-assets/editor.html',
    'package.json',
    'src-tauri/tauri.conf.json'
  ]
  // Windows 需要经 shell 解析 pnpm.cmd；参数均为固定相对路径。
  const result =
    process.platform === 'win32'
      ? run(['pnpm', ...args].join(' '), [], { shell: true })
      : run('pnpm', args)
  if (result.error?.code === 'ENOENT') throw new ReleaseError('找不到 pnpm，无法格式化发布文件')
  if (result.status !== 0)
    throw new ReleaseError(`格式化失败：\n${(result.stderr || result.stdout).trim()}`)
}

function commitAndPush(version, tag, branch) {
  const files = Object.values(PATHS)
  const add = git(['add', '--', ...files])
  if (add.status !== 0) throw new ReleaseError(`git add 失败：${(add.stderr || add.stdout).trim()}`)

  const commit = git(['commit', '-m', `chore(release): 发布 ${version}`])
  if (commit.status !== 0)
    throw new ReleaseError(`git commit 失败：${(commit.stderr || commit.stdout).trim()}`)
  console.log(`已提交：chore(release): 发布 ${version}`)

  const tagResult = git(['tag', '-a', tag, '-m', `Nons ${version}`])
  if (tagResult.status !== 0)
    throw new ReleaseError(`创建标签失败：${(tagResult.stderr || tagResult.stdout).trim()}`)
  console.log(`已创建标签：${tag}`)

  const pushBranch = git(['push', 'origin', branch])
  if (pushBranch.status !== 0)
    throw new ReleaseError(
      `推送分支失败：${(pushBranch.stderr || pushBranch.stdout).trim()}\n本地提交和标签已保留，可手动重试。`
    )
  console.log(`已推送分支：${branch}`)

  const pushTag = git(['push', 'origin', tag])
  if (pushTag.status !== 0)
    throw new ReleaseError(
      `推送标签失败：${(pushTag.stderr || pushTag.stdout).trim()}\n请手动执行 git push origin ${tag}。`
    )
  console.log(`已推送标签：${tag}`)
}

function parseArguments(argv) {
  const options = { yes: false, dryRun: false, help: false, positional: [] }
  const valueOptions = {
    '--subtitle': 'subtitle',
    '--platform': 'platform',
    '--release-kind': 'releaseKind',
    '--intro': 'intro'
  }
  for (let index = 0; index < argv.length; index++) {
    const argument = argv[index]
    if (argument === '--yes' || argument === '-y') options.yes = true
    else if (argument === '--dry-run') options.dryRun = true
    else if (argument === '--help' || argument === '-h') options.help = true
    else if (valueOptions[argument]) {
      const value = argv[++index]
      if (value === undefined) throw new ReleaseError(`${argument} 缺少值`)
      options[valueOptions[argument]] = value
    } else if (argument.startsWith('--') && argument.includes('=')) {
      const separator = argument.indexOf('=')
      const name = argument.slice(0, separator)
      if (!valueOptions[name]) throw new ReleaseError(`未知参数：${name}`)
      options[valueOptions[name]] = argument.slice(separator + 1)
    } else if (argument.startsWith('-')) throw new ReleaseError(`未知参数：${argument}`)
    else options.positional.push(argument)
  }
  if (options.positional.length > 1) throw new ReleaseError('只能提供一个版本号')
  options.version = options.positional[0]
  return options
}

async function ask(rl, label, defaultValue) {
  const hint = defaultValue ? ` [${defaultValue}]` : ''
  const answer = (await rl.question(`${label}${hint}: `)).trim()
  return answer || defaultValue || ''
}

async function main() {
  const options = parseArguments(process.argv.slice(2))
  if (options.help) {
    console.log(HELP)
    return
  }

  const interactive = Boolean(process.stdin.isTTY)
  if (!interactive && !options.yes) throw new ReleaseError('非交互环境请传入版本号并加 --yes')

  const headerSvg = await readFile(join(ROOT, PATHS.headerSvg), 'utf8')
  const current = {
    version: elementText(headerSvg, 'text', 'version'),
    subtitle: elementText(headerSvg, 'text', 'subtitle'),
    platform: elementText(headerSvg, 'text', 'platform'),
    releaseKind: elementText(headerSvg, 'text', 'release-kind')
  }

  let rl
  try {
    if (interactive) rl = createInterface({ input: process.stdin, output: process.stdout })
    const prompt = async (label, defaultValue) => (rl ? ask(rl, label, defaultValue) : defaultValue)

    const version = parseVersion(
      options.version ?? (await prompt('版本号（不带 v，例如 1.10.0）', ''))
    )
    const displayVersion = `v${version}`
    if ([...displayVersion].length > MAX_LENGTH.version)
      throw new ReleaseError(`版本号超过 ${MAX_LENGTH.version} 字：${displayVersion}`)
    const tag = displayVersion

    const currentVersion = (await readJsonFile(PATHS.packageJson)).version
    if (compareVersions(version, currentVersion) <= 0)
      throw new ReleaseError(`新版本 ${version} 必须高于当前版本 ${currentVersion}`)

    const branch = verifyPreconditions(tag, options.dryRun)

    const subtitle = checkText(
      'subtitle',
      '副标题',
      options.subtitle ?? (await prompt('副标题', current.subtitle))
    )
    const platform = checkText(
      'platform',
      '平台',
      options.platform ?? (await prompt('平台', current.platform))
    )
    const releaseKind = checkText(
      'releaseKind',
      '发布类型',
      options.releaseKind ?? (await prompt('发布类型', current.releaseKind))
    )
    const suggestion = suggestedIntro(version, subtitle, platform)
    let intro = options.intro
    if (intro === undefined) {
      intro = await prompt('版本概述（回车使用建议内容）', suggestion)
    }
    intro = intro.trim()
    if (!intro) throw new ReleaseError('版本概述不能为空')

    const values = {
      version,
      displayVersion,
      subtitle,
      platform,
      releaseKind,
      intro,
      date: today()
    }

    const [
      packageJsonSource,
      tauriConfSource,
      cargoTomlSource,
      cargoLockSource,
      changelogSource,
      editorSource
    ] = await Promise.all([
      readFile(join(ROOT, PATHS.packageJson), 'utf8'),
      readFile(join(ROOT, PATHS.tauriConf), 'utf8'),
      readFile(join(ROOT, PATHS.cargoToml), 'utf8'),
      readFile(join(ROOT, PATHS.cargoLock), 'utf8'),
      readFile(join(ROOT, PATHS.changelog), 'utf8'),
      readFile(join(ROOT, PATHS.editorHtml), 'utf8')
    ])

    const outputs = {
      [PATHS.packageJson]: updateJsonVersion(packageJsonSource, version),
      [PATHS.tauriConf]: updateJsonVersion(tauriConfSource, version),
      [PATHS.cargoToml]: updateCargoToml(cargoTomlSource, version),
      [PATHS.cargoLock]: updateCargoLock(cargoLockSource, version),
      [PATHS.headerSvg]: updateHeaderSvg(headerSvg, values),
      [PATHS.editorHtml]: updateEditorHtml(editorSource, values)
    }
    const changelog = updateChangelog(changelogSource, values)
    outputs[PATHS.changelog] = changelog.text
    if (changelog.empty) {
      console.warn('警告：Unreleased 没有正文，版本条目只包含概述。')
    }

    const temporary = await mkdtemp(join(tmpdir(), 'nons-release-'))
    const previewPng = join(temporary, 'header.png')
    await renderHeaderPng(outputs[PATHS.headerSvg], previewPng)

    console.log(`\n版本：${displayVersion}`)
    console.log(`副标题：${subtitle}`)
    console.log(`平台：${platform}`)
    console.log(`发布类型：${releaseKind}`)
    console.log(`概述：${intro}`)
    console.log(
      `\nCHANGELOG：${changelog.inserted ? '在 Unreleased 下插入版本条目' : '已有版本条目，保持不变'}`
    )
    console.log(`预览头图：${previewPng}`)
    console.log(`\n将修改：\n  ${Object.values(PATHS).join('\n  ')}`)

    if (options.dryRun) {
      console.log('\n--dry-run：未修改仓库，未提交、未打标签。')
      return
    }

    if (!options.yes) {
      const confirmed = await ask(rl, '确认提交、推送分支并推送标签？(y/N)', '')
      if (!/^y(es)?$/i.test(confirmed)) {
        await rm(temporary, { recursive: true, force: true })
        console.log('已取消，未修改任何文件。')
        return
      }
    }

    await Promise.all(
      Object.entries(outputs)
        .filter(([path]) => path !== PATHS.headerPng)
        .map(([path, content]) => writeFile(join(ROOT, path), content, 'utf8'))
    )
    await copyFile(previewPng, join(ROOT, PATHS.headerPng))
    await rm(temporary, { recursive: true, force: true })

    formatFiles()
    await validateWithReleaseScript(tag)
    commitAndPush(version, tag, branch)

    console.log(`\n发布完成。CI：https://github.com/Miaoyww/Nons/actions`)
  } finally {
    rl?.close()
  }
}

const isMain = process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href
if (isMain) {
  main().catch((error) => {
    console.error(`\n发布中止：${error.message}`)
    process.exitCode = 1
  })
}

export {
  updateCargoLock,
  updateCargoToml,
  updateChangelog,
  updateEditorHtml,
  updateHeaderSvg,
  updateJsonVersion
}
