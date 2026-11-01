import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { fileURLToPath } from 'node:url'

test('welcome adapter and plugin lists scroll to their last entry while navigation stays visible', async (t) => {
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
  for (const viewport of [
    { width: 1200, height: 720 },
    { width: 800, height: 600 },
    { width: 390, height: 720 },
    { width: 1200, height: 500 }
  ]) {
    await page.setViewportSize(viewport)
    await page.goto(`${server.resolvedUrls.local[0]}tests/browser/welcome-page.html`, {
      waitUntil: 'domcontentloaded',
      timeout: 120000
    })
    await page.getByRole('heading', { name: '欢迎使用 NonsPlayer' }).waitFor()
    assert.equal(await page.locator('.welcome-footer').count(), 0)
    const intro = page.locator('.welcome-intro')
    const introBounds = await intro.boundingBox()
    assert.ok(introBounds.y >= 0 && introBounds.y + introBounds.height <= viewport.height)
    await page.getByRole('button', { name: '开始设置', exact: true }).click()
    await page.getByRole('heading', { name: '隐私协议' }).waitFor()
    await page.locator('.privacy-policy table').waitFor()
    const next = page.getByRole('button', { name: '下一步', exact: true })
    assert.equal(await next.isDisabled(), true)
    assert.equal(
      await page.getByRole('button', { name: '跳过引导', exact: true }).isDisabled(),
      true
    )
    await page.getByRole('checkbox', { name: '我已阅读并同意《隐私协议》' }).check()
    assert.equal(await next.isDisabled(), false)
    await next.click()
    for (const [noun, name] of [
      ['适配器', '音乐来源'],
      ['插件', '音乐工具']
    ]) {
      const store = page.locator(`section[aria-label="${noun}商店"]`)
      await store.getByText(`${name} 1`, { exact: true }).waitFor()
      const content = page.locator('.welcome-step-content')
      const bounds = await content.boundingBox()
      await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + bounds.height - 32)
      for (let batch = 0; batch < 4; batch++) {
        const count = await store.locator('.extension-store-entries > article').count()
        await page.mouse.wheel(0, 100000)
        await page.waitForFunction(
          (count) =>
            document.querySelectorAll('.extension-store-entries > article').length > count ||
            document.querySelectorAll('.extension-store-entries > article').length === 55,
          count,
          { timeout: 5000 }
        )
        if (await store.getByText(`${name} 55`, { exact: true }).count()) break
      }
      await page.mouse.wheel(0, 100000)
      await page.waitForFunction(
        () => {
          const content = document.querySelector('.welcome-step-content')
          const last = content.querySelector('.extension-store-entries > article:last-child')
          const bounds = last.getBoundingClientRect()
          const viewport = content.getBoundingClientRect()
          return bounds.top >= viewport.top && bounds.bottom <= viewport.bottom
        },
        undefined,
        { timeout: 3000 }
      )
      assert.equal(await store.getByText(`${name} 55`, { exact: true }).count(), 1)
      assert.equal(
        await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth),
        true
      )
      const footer = await page.locator('.welcome-footer').boundingBox()
      assert.ok(footer.y >= 0 && footer.y + footer.height <= viewport.height)
      await page.getByRole('button', { name: '下一步', exact: true }).click()
    }
  }
  assert.deepEqual(errors, [])
})
