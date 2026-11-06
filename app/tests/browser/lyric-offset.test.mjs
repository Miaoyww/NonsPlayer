import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { fileURLToPath } from 'node:url'

test('lyric offset controls appear on lyrics hover and adjust seconds vertically', async (t) => {
  const server = await createServer({
    root: fileURLToPath(new URL('../../', import.meta.url)),
    mode: 'test',
    server: { host: '127.0.0.1', port: 0, strictPort: false }
  })
  t.after(() => server.close())
  await server.listen()
  let browser
  const launchErrors = []
  for (const channel of [process.env.PLAYWRIGHT_CHANNEL, 'msedge', 'chrome']) {
    try {
      browser = await chromium.launch({ headless: true, channel })
      break
    } catch (cause) {
      launchErrors.push(cause.message)
    }
  }
  assert.ok(browser, launchErrors.join('\n'))
  t.after(() => browser.close())
  const page = await browser.newPage({ viewport: { width: 900, height: 650 } })
  const errors = []
  page.on('pageerror', (cause) => errors.push(cause.message))
  await page.goto(`${server.resolvedUrls.local[0]}tests/browser/lyric-offset.html`, {
    waitUntil: 'domcontentloaded',
    timeout: 60000
  })
  const controls = page.getByRole('group', { name: '单曲额外歌词偏移' })
  const plus = page.getByRole('button', { name: '歌词延后 0.1 秒' })
  const minus = page.getByRole('button', { name: '歌词提前 0.1 秒' })
  const value = controls.locator('output')
  await controls.waitFor({ timeout: 60000 })
  await page.mouse.move(20, 620)
  assert.equal(await controls.evaluate((node) => getComputedStyle(node).opacity), '0')
  const region = page.locator('.now-playing-panel-region')
  const before = await region.boundingBox()
  assert.equal(before.y, 48)
  assert.ok(Math.abs(before.x + before.width - 900) < 1)
  assert.ok(Math.abs(before.y + before.height - 650) < 1)
  await page.getByText('Life goes on', { exact: true }).hover()
  await page.waitForFunction(
    () => getComputedStyle(document.querySelector('.lyric-offset-control')).opacity === '1'
  )
  assert.deepEqual(await region.boundingBox(), before)
  for (const point of [
    { x: 899, y: 50 },
    { x: 899, y: 648 },
    { x: before.x + 2, y: 50 },
    { x: before.x + 2, y: 648 }
  ]) {
    await page.mouse.move(point.x, point.y)
    await page.waitForTimeout(180)
    assert.equal(await controls.evaluate((node) => getComputedStyle(node).opacity), '1')
  }
  const [plusBox, valueBox, minusBox] = await Promise.all([
    plus.boundingBox(),
    value.boundingBox(),
    minus.boundingBox()
  ])
  assert.ok(plusBox.y + plusBox.height <= valueBox.y)
  assert.ok(valueBox.y + valueBox.height <= minusBox.y)
  assert.ok(minusBox.x + minusBox.width >= 896)
  assert.equal(await value.evaluate((node) => getComputedStyle(node).padding), '2px 4px')
  await plus.click()
  assert.equal(await value.textContent(), '+0.1 s')
  await minus.click()
  await minus.click()
  assert.equal(await value.textContent(), '-0.1 s')
  await page.mouse.move(20, 620)
  await page.waitForFunction(
    () => getComputedStyle(document.querySelector('.lyric-offset-control')).opacity === '0'
  )
  await page.getByText('Life goes on', { exact: true }).hover()
  await value.hover()
  await page.waitForTimeout(200)
  assert.equal(await controls.evaluate((node) => getComputedStyle(node).opacity), '1')
  await page.screenshot({ path: 'C:/Users/miaom/AppData/Local/Temp/nons-lyric-offset-hover.png' })
  await plus.evaluate((node) => node.blur())
  await minus.evaluate((node) => node.blur())
  await page.mouse.move(20, 620)
  await page.waitForFunction(
    () => getComputedStyle(document.querySelector('.lyric-offset-control')).opacity === '0'
  )
  await page.keyboard.press('Tab')
  await plus.focus()
  await page.waitForFunction(
    () => getComputedStyle(document.querySelector('.lyric-offset-control')).opacity === '1'
  )
  await page.getByRole('button', { name: '上限', exact: true }).click()
  assert.equal(await plus.isDisabled(), true)
  assert.equal(await value.textContent(), '+60.0 s')
  await page.getByRole('button', { name: '下限', exact: true }).click()
  assert.equal(await minus.isDisabled(), true)
  assert.equal(await value.textContent(), '-60.0 s')
  assert.deepEqual(errors, [])
})
