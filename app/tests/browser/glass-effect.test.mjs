import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { fileURLToPath } from 'node:url'

test('glass effect switch toggles blur and survives reload', async (t) => {
  const server = await createServer({
    root: fileURLToPath(new URL('../../', import.meta.url)),
    server: { host: '127.0.0.1', port: 0, strictPort: false }
  })
  t.after(() => server.close())
  await server.listen()
  let browser
  for (const channel of [process.env.PLAYWRIGHT_CHANNEL, 'msedge', 'chrome']) {
    try {
      browser = await chromium.launch({ headless: true, channel })
      break
    } catch {}
  }
  assert.ok(browser, 'A Playwright Chromium, Edge or Chrome installation is required')
  t.after(() => browser.close())
  const page = await browser.newPage()
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  const url = `${server.resolvedUrls.local[0]}tests/browser/glass-effect.html`
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 120000 })

  const surface = page.getByTestId('surface')
  const toggle = page.getByRole('switch', { name: '毛玻璃效果' })
  await toggle.waitFor()
  assert.equal(await page.evaluate(() => document.documentElement.dataset.glassEffect), undefined)
  assert.match(await surface.evaluate((node) => getComputedStyle(node).backdropFilter), /blur/)

  await toggle.click()
  assert.equal(await page.evaluate(() => document.documentElement.dataset.glassEffect), 'off')
  assert.equal(await surface.evaluate((node) => getComputedStyle(node).backdropFilter), 'none')
  assert.equal(await page.evaluate(() => localStorage.getItem('nons-glass-effect')), 'off')

  await page.reload({ waitUntil: 'domcontentloaded' })
  await toggle.waitFor()
  assert.equal(await page.evaluate(() => document.documentElement.dataset.glassEffect), 'off')
  assert.equal(await surface.evaluate((node) => getComputedStyle(node).backdropFilter), 'none')

  await toggle.click()
  assert.equal(await page.evaluate(() => document.documentElement.dataset.glassEffect), undefined)
  assert.match(await surface.evaluate((node) => getComputedStyle(node).backdropFilter), /blur/)
  assert.deepEqual(errors, [])
})
