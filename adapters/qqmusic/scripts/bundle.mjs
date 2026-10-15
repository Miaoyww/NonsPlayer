import { build } from 'esbuild'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
import { resolve, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { builtinModules } from 'node:module'
const project = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const upstream = resolve(project, 'node_modules/@yakult-green-tea/qq-music-api/dist-esm/src')
const crypto = resolve(project, 'src/runtime/crypto.js').replaceAll('\\', '/')
export async function bundle(output = resolve(project, 'dist')) {
  await mkdir(output, { recursive: true })
  const result = await build({
    absWorkingDir: project,
    entryPoints: ['src/index.js'],
    outfile: resolve(output, 'guest.js'),
    bundle: true,
    format: 'esm',
    platform: 'browser',
    target: 'es2022',
    metafile: true,
    minify: false,
    inject: [resolve(project, 'src/runtime/inject.js')],
    define: { 'globalThis.crypto': 'runtimeCrypto' },
    external: [
      'nons:host/transport@1.0.0',
      'nons:crypto/primitives@1.0.0',
      'nons:plugin/host@1.0.0'
    ],
    plugins: [
      {
        name: 'neutral-protocol',
        setup(builder) {
          builder.onResolve({ filter: /^qq-protocol\// }, (args) => ({
            path: resolve(upstream, 'services', args.path.slice('qq-protocol/'.length))
          }))
          builder.onResolve({ filter: /^qq-fetch-auth$/ }, () => ({
            path: resolve(upstream, 'serverless/fetchAuthHttpClient.js')
          }))
          builder.onLoad({ filter: /[/\\]util[/\\]logger\.js$/ }, () => ({
            contents:
              'const ignore = () => {}; export const logger = { debug: ignore, info: ignore, warn: ignore, error: ignore };',
            loader: 'js'
          }))
          builder.onLoad({ filter: /[/\\]auth[/\\]androidDevice\.js$/ }, async (args) => {
            let contents = await readFile(args.path, 'utf8')
            const originalMd5 =
              "export const md5 = (...values) => md5Digest(new TextEncoder().encode(values.join('')));"
            const rsaStart = contents.indexOf(
              'export const rsaesPkcs1Encrypt = (message, modulusHex, exponent) => {'
            )
            const rsaEnd = contents.indexOf('\nconst rsaEncryptQimeiKey = ', rsaStart)
            if (!contents.includes(originalMd5) || rsaStart < 0 || rsaEnd < 0)
              throw new Error('Pinned protocol crypto hooks changed; inspect before upgrading')
            contents =
              contents.slice(0, rsaStart) +
              'export const rsaesPkcs1Encrypt = protocolRsaEncrypt;\n' +
              contents.slice(rsaEnd)
            contents =
              `import { protocolMd5, protocolRsaEncrypt } from ${JSON.stringify(crypto)};\n` +
              contents.replace(originalMd5, 'export const md5 = protocolMd5;')
            return { contents, loader: 'js' }
          })
          builder.onLoad({ filter: /[/\\]auth[/\\]qrLogin\.js$/ }, async (args) => ({
            contents:
              (await readFile(args.path, 'utf8')) +
              '\nexport const adapterMusicu = (http, auth, module, method, param) => callMusicu(http, auth.device, "adapter-read", module, method, param, auth.credential);\n' +
              'export const adapterRefresh = async (http, auth) => { const c = auth.credential; const credential = credentialFrom(await callMusicu(http, auth.device, "credential-refresh", "music.login.LoginServer", "Login", { openid: stringOf(c.openid), access_token: stringOf(c.access_token), refresh_token: stringOf(c.refresh_token), expired_in: c.expired_at, str_musicid: credentialMusicId(c), musicid: c.musicid, musickey: c.musickey, unionid: stringOf(c.unionid), refresh_key: stringOf(c.refresh_key), loginMode: 2 }, c, { tmeLoginType: c.loginType }), c.loginType); return { ...auth, credential, expiresAt: authSessionExpiryAt(credential, Date.now()) }; };\n',
            loader: 'js'
          }))
        }
      }
    ]
  })
  const builtins = new Set(builtinModules.flatMap((value) => [value, `node:${value}`]))
  for (const [path, input] of Object.entries(result.metafile.inputs)) {
    if (/[/\\](axios|koa|electron|server|config)[/\\]/.test(path))
      throw new Error(`Non-neutral dependency: ${path}`)
    for (const value of input.imports)
      if (builtins.has(value.path)) throw new Error(`Node builtin reached guest: ${value.path}`)
  }
  for (const output of Object.values(result.metafile.outputs))
    for (const value of output.imports) {
      if (
        ![
          'nons:host/transport@1.0.0',
          'nons:crypto/primitives@1.0.0',
          'nons:plugin/host@1.0.0'
        ].includes(value.path)
      )
        throw new Error(`Unexpected guest import: ${value.path}`)
    }
  const source = await readFile(resolve(output, 'guest.js'), 'utf8')
  if (/\bprocess\.(?:env|versions)|\b(?:modPow|md5Digest)\b/.test(source))
    throw new Error('Guest retained runtime or hand-written crypto dependency')
  await writeFile(resolve(output, 'closure.json'), JSON.stringify(result.metafile, null, 2))
  await writeFile(
    resolve(output, 'LICENSE.qq-music-api'),
    await readFile(resolve(upstream, '../../LICENSE'))
  )
  return resolve(output, 'guest.js')
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url))
  console.log(await bundle())
