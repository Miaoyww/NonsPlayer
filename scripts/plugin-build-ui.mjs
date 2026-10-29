import { build } from '../app/node_modules/vite/dist/node/index.js'
import { readFile } from 'node:fs/promises'
import { resolve } from 'node:path'

export async function buildFrontend(project, out, filename) {
  const external = new Map([
    ['react', './_host/react.mjs'],
    ['react/jsx-runtime', './_host/jsx-runtime.mjs'],
    ['@nonsplayer/plugin-sdk', './_host/sdk.mjs'],
    ['@app/plugin-sdk', './_host/sdk.mjs']
  ])
  await build({
    configFile: false,
    root: project,
    logLevel: 'warn',
    plugins: [
      {
        name: 'plugin-host-singletons',
        resolveId(id) {
          if (external.has(id)) return { id: external.get(id), external: true }
          if (id.includes('/src/') || id.startsWith('@/'))
            throw new Error('Plugin must use @nonsplayer/plugin-sdk, not host sources')
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
        external: [...external.keys(), ...external.values()],
        output: {
          paths: Object.fromEntries(external),
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
    /from\s*["'](?:react|react\/jsx-runtime|@nonsplayer\/plugin-sdk|@app\/plugin-sdk)["']/.test(
      artifact
    )
  )
    throw new Error('Plugin bundled React or retained unresolved host imports')
}
