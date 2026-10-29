import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { buildPlugin } from './build.mjs'
import { createPlugin } from './create.mjs'
import { formatErrors, SDK_VERSION, validateManifest } from './manifest.mjs'

const help = `用法：nons-plugin <命令> [参数]

命令：
  create <目录> [--id <id>] [--name <名称>] [--description <描述>]
      从模板创建前端插件项目
  build [目录] [--out <目录>] [--zip <文件>] [--no-zip] [--no-typecheck]
      检查、打包并校验插件，默认输出 <目录>/dist 与 <id>-<版本>.zip
  validate <manifest.json> [--root <目录>]
      校验 Manifest；提供 --root 时同时检查打包文件

SDK ${SDK_VERSION}，WIT 与 Manifest schema 随包提供：
  node_modules/@nonsplayer/plugin-sdk/wit/plugin.wit
  node_modules/@nonsplayer/plugin-sdk/schemas/manifest.schema.json`

const valueFlags = new Set(['out', 'zip', 'id', 'name', 'description', 'root'])
const booleanFlags = new Set(['no-zip', 'no-typecheck', 'help', 'version'])

function parse(argv) {
  const positional = []
  const flags = {}
  for (let index = 0; index < argv.length; index++) {
    const argument = argv[index]
    if (!argument.startsWith('--')) {
      positional.push(argument)
      continue
    }
    const [name, inline] = argument.slice(2).split('=', 2)
    if (valueFlags.has(name)) {
      const value = inline ?? argv[++index]
      if (!value) throw new Error(`--${name} 缺少取值`)
      flags[name] = value
    } else if (booleanFlags.has(name) && inline === undefined) flags[name] = true
    else throw new Error(`未知参数：${argument}`)
  }
  return { positional, flags }
}

export async function run(argv) {
  const { positional, flags } = parse(argv)
  if (flags.version) {
    console.log(SDK_VERSION)
    return
  }
  if (flags.help || positional.length === 0) {
    console.log(help)
    return
  }
  const [command, ...rest] = positional
  if (command === 'create') {
    if (rest.length > 1) throw new Error('create 只接受一个目录')
    const { target, id } = await createPlugin(rest[0] ?? '.', flags)
    console.log(`已创建插件 ${id}：${target}`)
    console.log('下一步：pnpm install && pnpm build')
    return
  }
  if (command === 'build') {
    if (rest.length > 1) throw new Error('build 只接受一个目录')
    const result = await buildPlugin({
      project: rest[0] ?? process.cwd(),
      out: flags.out,
      zip: flags['no-zip'] ? false : undefined,
      zipPath: flags.zip,
      typecheck: !flags['no-typecheck']
    })
    console.log(`插件产物：${result.out}`)
    if (result.zip) console.log(`插件 ZIP：${result.zip}`)
    return
  }
  if (command === 'validate') {
    if (rest.length !== 1) throw new Error('validate 需要一个 manifest.json 路径')
    const manifestPath = resolve(rest[0])
    const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
    const root = flags.root ? resolve(flags.root) : null
    const errors = await validateManifest(manifest, { root })
    if (errors.length) throw new Error(`Manifest 校验失败：\n${formatErrors(errors)}`)
    console.log(`Manifest 校验通过：${manifest.id}@${manifest.version}`)
    return
  }
  throw new Error(`未知命令：${command}`)
}
