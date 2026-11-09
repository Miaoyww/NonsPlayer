import assert from 'node:assert/strict'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'
import { chromium } from 'playwright'
import { createServer } from 'vite'

async function launchBrowser() {
  const channels = process.env.PLAYWRIGHT_CHANNEL
    ? [process.env.PLAYWRIGHT_CHANNEL]
    : [undefined, 'msedge', 'chrome']
  const errors = []
  for (const channel of channels) {
    try {
      return await chromium.launch({ headless: true, channel })
    } catch (error) {
      errors.push(error.message)
    }
  }
  throw new Error(
    `Install Chromium with pnpm exec playwright install chromium.\n${errors.join('\n')}`
  )
}

test('network service status preserves drafts and lets an invalid port close the service', async (t) => {
  const root = fileURLToPath(new URL('../../', import.meta.url))
  const stub = fileURLToPath(new URL('./network-service-stubs.tsx', import.meta.url))
  const server = await createServer({
    root,
    mode: 'test',
    server: { host: '127.0.0.1', port: 0, strictPort: false },
    resolve: {
      alias: [
        { find: '@tauri-apps/api/core', replacement: stub },
        { find: '@tauri-apps/api/event', replacement: stub },
        { find: '@tauri-apps/plugin-clipboard-manager', replacement: stub },
        { find: '@tauri-apps/plugin-opener', replacement: stub },
        { find: '@/lib/player', replacement: stub }
      ]
    }
  })
  t.after(() => server.close())
  await server.listen()
  const browser = await launchBrowser()
  t.after(() => browser.close())
  const page = await browser.newPage({ viewport: { width: 1100, height: 900 } })
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${server.resolvedUrls.local[0]}tests/browser/network-service.html`, {
    waitUntil: 'domcontentloaded',
    timeout: 60_000
  })
  const port = page.locator('#open-api-port')
  await port.waitFor()

  await port.fill('47000')
  await page.evaluate(() => window.networkServiceFixture.emitStatus({ connections: 2 }))
  assert.equal(await port.inputValue(), '47000')
  await page.getByText('2 个连接').waitFor()

  await port.fill('0')
  await page.getByLabel('启用开放 API').click()
  await page.waitForFunction(() =>
    window.networkServiceFixture.calls.some(([command]) => command === 'open_api_configure')
  )
  const configure = await page.evaluate(() =>
    window.networkServiceFixture.calls.find(([command]) => command === 'open_api_configure')
  )
  assert.deepEqual(configure[1], {
    config: { enabled: false, port: 46321, lan: false }
  })
  assert.deepEqual(errors, [])
})
