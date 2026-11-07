import { defineConfig, loadEnv, type Plugin } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import react from '@vitejs/plugin-react'
import process from 'node:process'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
const host = process.env.TAURI_DEV_HOST
const root = fileURLToPath(new URL('.', import.meta.url))

const telemetrySummaryPath = '/__dev/telemetry-summary'
const telemetrySummaryEndpoint = 'https://nons-api.miaoyww.top/v1/summary'
const telemetryRetryDelays = [0, 2_000, 5_000]

interface TelemetrySummary {
  usage_count: string
  play_count: string
  generated_at: string
}

class TelemetryRequestError extends Error {
  constructor(
    message: string,
    readonly retryable: boolean
  ) {
    super(message)
  }
}

function readCount(value: unknown) {
  if (typeof value === 'number')
    return Number.isSafeInteger(value) && value >= 0 ? String(value) : undefined
  return typeof value === 'string' && /^\d+$/.test(value) ? value : undefined
}

function parseTelemetrySummary(value: unknown): TelemetrySummary | undefined {
  if (!value || typeof value !== 'object') return
  const summary = value as Record<string, unknown>
  const usageCount = readCount(summary.usage_count)
  const playCount = readCount(summary.play_count)
  const generatedAt = typeof summary.generated_at === 'string' ? summary.generated_at.trim() : ''
  if (!usageCount || !playCount || !generatedAt) return
  return { usage_count: usageCount, play_count: playCount, generated_at: generatedAt }
}

async function requestTelemetrySummary(key: string) {
  const response = await fetch(telemetrySummaryEndpoint, {
    headers: { 'X-Stats-Key': key },
    signal: AbortSignal.timeout(10_000)
  })
  if (!response.ok) {
    throw new TelemetryRequestError(
      `HTTP ${response.status}`,
      response.status === 429 || response.status >= 500
    )
  }
  const summary = parseTelemetrySummary(await response.json().catch(() => undefined))
  if (!summary) throw new TelemetryRequestError('响应格式无效', false)
  return summary
}

async function loadTelemetrySummary(key: string) {
  let lastError: unknown
  for (const delay of telemetryRetryDelays) {
    if (delay > 0) await new Promise((resolve) => setTimeout(resolve, delay))
    try {
      return await requestTelemetrySummary(key)
    } catch (error) {
      lastError = error
      if (error instanceof TelemetryRequestError && !error.retryable) break
    }
  }
  throw lastError
}

function devTelemetrySummary(mode: string): Plugin {
  return {
    name: 'nons-dev-telemetry-summary',
    apply: 'serve',
    configureServer(server) {
      const key = loadEnv(mode, server.config.root, '').NONS_TELEMETRY_STATS_KEY?.trim()
      let summary: TelemetrySummary | undefined
      const loaded = key
        ? loadTelemetrySummary(key)
            .then((value) => {
              summary = value
              server.config.logger.info(`[telemetry] 已加载开发统计快照（${value.generated_at}）`)
            })
            .catch((error: unknown) => {
              server.config.logger.warn(
                `[telemetry] 开发统计快照获取失败：${error instanceof Error ? error.message : String(error)}`
              )
            })
        : undefined

      if (!key) {
        server.config.logger.warn(
          '[telemetry] 未配置 NONS_TELEMETRY_STATS_KEY（app/.env.local），关于页将显示“暂无统计数据”。'
        )
      }

      server.middlewares.use(telemetrySummaryPath, async (request, response, next) => {
        if (request.method !== 'GET') {
          next()
          return
        }
        response.setHeader('Cache-Control', 'no-store')
        if (!key) {
          response.statusCode = 204
          response.end()
          return
        }
        await loaded
        if (!summary) {
          response.statusCode = 502
          response.setHeader('Content-Type', 'application/json; charset=utf-8')
          response.end(JSON.stringify({ error: 'unavailable' }))
          return
        }
        response.statusCode = 200
        response.setHeader('Content-Type', 'application/json; charset=utf-8')
        response.end(JSON.stringify(summary))
      })
    }
  }
}

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [react(), tailwindcss(), devTelemetrySummary(mode)],
  build: {
    rollupOptions: {
      input: {
        main: resolve(root, 'index.html'),
        desktopLyrics: resolve(root, 'desktop-lyrics.html')
      }
    }
  },
  resolve: {
    alias: {
      '@': decodeURIComponent(new URL('./src', import.meta.url).pathname).replace(/^\/(\w:)/, '$1')
    }
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    ws: host
      ? {
          protocol: 'ws',
          host: host,
          port: 1421,
          clientPort: 1421
        }
      : undefined,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**', '**/dist/**', '**/target/**']
    },
    fs: {
      // 4. PRIVACY.md sits at the repository root and is imported as text.
      allow: ['.', '..']
    }
  }
}))
