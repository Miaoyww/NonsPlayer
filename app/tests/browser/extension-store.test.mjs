import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { fileURLToPath } from 'node:url'

test('extension store fits narrow settings, switches index sources and keeps confirmation reachable', async (t) => {
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
  for (const dark of [false, true]) {
    for (const width of [360, 800]) {
      await page.setViewportSize({ width, height: 720 })
      await page.goto(`${server.resolvedUrls.local[0]}tests/browser/extension-store.html`, {
        waitUntil: 'domcontentloaded',
        timeout: 120000
      })
      await page.evaluate((dark) => document.documentElement.classList.toggle('dark', dark), dark)
      await page.getByText('音乐工具 1', { exact: true }).waitFor()
      await page.getByRole('contentinfo').scrollIntoViewIfNeeded()
      assert.equal(
        await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
        true
      )
      await page.getByText('音乐工具 1', { exact: true }).scrollIntoViewIfNeeded()
      const store = page.locator('section[aria-label="插件商店"]')
      await page.screenshot({
        path: `${process.env.TEMP}/nons-extension-store-${width}-${dark ? 'dark' : 'light'}.png`,
        fullPage: true
      })
      assert.equal(await store.count(), 1)
      await page.getByRole('combobox', { name: '索引来源' }).click()
      await page.getByRole('option', { name: 'GitHub meta', exact: true }).click()
      await page.getByText(/当前索引：GitHub meta/).waitFor()
      await page.getByRole('button', { name: '安装', exact: true }).first().click()
      const review = page.getByRole('dialog')
      await review.getByRole('button', { name: '确认安装', exact: true }).scrollIntoViewIfNeeded()
      assert.equal(
        await review.getByRole('button', { name: '确认安装', exact: true }).isDisabled(),
        true
      )
      assert.equal(
        await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
        true
      )
      await review.getByRole('checkbox').check()
      assert.equal(
        await review.getByRole('button', { name: '确认安装', exact: true }).isEnabled(),
        true
      )
      await page.screenshot({
        path: `${process.env.TEMP}/nons-extension-confirm-${width}-${dark ? 'dark' : 'light'}.png`
      })
      await review.getByRole('button', { name: '取消', exact: true }).click()
      await page.getByRole('dialog').waitFor({ state: 'hidden' })
    }
  }
  assert.deepEqual(errors, [])
})
