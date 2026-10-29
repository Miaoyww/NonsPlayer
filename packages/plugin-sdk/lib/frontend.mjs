import { build } from 'vite'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

const hostModules = new Map([
  ['react', './_host/react.mjs'],
  ['react/jsx-runtime', './_host/jsx-runtime.mjs'],
  ['@nonsplayer/plugin-sdk', './_host/sdk.mjs'],
  ['@app/plugin-sdk', './_host/sdk.mjs']
])
const unresolved =
  /from\s*["'](?:react|react\/jsx-runtime|@nonsplayer\/plugin-sdk|@app\/plugin-sdk)["']/

export async function buildFrontend(project, out, filename) {
  await build({
    configFile: false,
    root: project,
    logLevel: 'warn',
    plugins: [
      {
        name: 'plugin-host-singletons',
        resolveId(id) {
          if (hostModules.has(id)) return { id: hostModules.get(id), external: true }
          if (id.includes('/src/') || id.startsWith('@/'))
            throw new Error('插件不得引用宿主源码，请使用 @nonsplayer/plugin-sdk')
        },
        renderChunk(code, chunk) {
          // External paths are emitted verbatim; lazy chunks live below assets/.
          const depth = chunk.fileName.split('/').length - 1
          if (!depth) return null
          return {
            code: code.replace(/(["'])\.\/_host\//g, `$1${'../'.repeat(depth)}_host/`),
            map: null
          }
        }
      }
    ],
    oxc: { jsx: { runtime: 'automatic' } },
    build: {
      outDir: out,
      emptyOutDir: false,
      lib: {
        entry: resolve(project, 'frontend/index.tsx'),
        formats: ['es'],
        fileName: () => filename
      },
      rolldownOptions: {
        external: [...hostModules.keys(), ...hostModules.values()],
        output: {
          paths: Object.fromEntries(hostModules),
          chunkFileNames: 'assets/[name]-[hash].mjs',
          assetFileNames: 'assets/[name]-[hash][extname]'
        }
      }
    }
  })
  const artifact = await readFile(resolve(out, filename), 'utf8')
  if (
    artifact.includes('react.transitional.element') ||
    artifact.includes('Invalid hook call') ||
    unresolved.test(artifact)
  )
    throw new Error('插件打包了 React 或保留了未解析的宿主导入')
}
