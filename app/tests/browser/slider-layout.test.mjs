import assert from 'node:assert/strict'
import { test } from 'node:test'
import { chromium } from 'playwright'
import { createServer } from 'vite'
import { fileURLToPath } from 'node:url'

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

test('playback sliders stay inside scroll containers and fill the exact numeric range', async (t) => {
  const root = fileURLToPath(new URL('../../', import.meta.url))
  const server = await createServer({
    root,
    server: { host: '127.0.0.1', port: 0, strictPort: false }
  })
  t.after(() => server.close())
  await server.listen()
  const browser = await launchBrowser()
  t.after(() => browser.close())
  const page = await browser.newPage({ viewport: { width: 1400, height: 1050 } })
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto(`${server.resolvedUrls.local[0]}tests/browser/slider-layout.html`)
  await page.locator('#progress input').waitFor()

  for (const width of [280, 380, 570, 760]) {
    await page.locator('#details').evaluate((element, width) => {
      element.style.width = `${width}px`
    }, width)
    for (const value of [0, 1, 50, 98, 99, 100]) {
      await page.locator('#value').fill(String(value))
      await page.waitForFunction(
        (value) => document.querySelector('#progress input').value === String(value),
        value
      )
      await page.evaluate(
        () => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
      )
      const measurements = await page.evaluate(() => {
        const details = document.querySelector('#details')
        const vertical = document.querySelector('#vertical-container')
        const sliders = ['#progress', '#volume', '#vertical'].map((selector) => {
          const slider = document.querySelector(selector)
          const track = slider.querySelector('[data-slot="slider-track"]').getBoundingClientRect()
          const thumb = slider.querySelector('[data-slot="slider-thumb"]').getBoundingClientRect()
          const fill = slider.querySelector('[data-slot="slider-range"]').getBoundingClientRect()
          const control = slider
            .querySelector('[data-slot="slider-thumb"]')
            .parentElement.getBoundingClientRect()
          const isVertical = selector === '#vertical'
          return {
            selector,
            fillRatio: isVertical ? fill.height / track.height : fill.width / track.width,
            inside: isVertical
              ? thumb.top >= control.top - 0.1 && thumb.bottom <= control.bottom + 0.1
              : thumb.left >= control.left - 0.1 && thumb.right <= control.right + 0.1
          }
        })
        return {
          width: details.clientWidth,
          scrollWidth: details.scrollWidth,
          verticalHeight: vertical.clientHeight,
          verticalScroll: vertical.scrollHeight,
          sliders
        }
      })
      const context = `width=${width}, value=${value}: ${JSON.stringify(measurements)}`
      assert.equal(measurements.scrollWidth, measurements.width, context)
      assert.equal(measurements.verticalScroll, measurements.verticalHeight, context)
      for (const slider of measurements.sliders) {
        assert.ok(slider.inside, context)
        assert.ok(Math.abs(slider.fillRatio - value / 100) < 0.002, context)
      }
    }
  }
  const progress = page.locator('#progress input')
  await progress.focus()
  await progress.press('Home')
  await progress.press('End')
  assert.equal(await progress.inputValue(), '100')
  await page.locator('#progress [data-slot="slider-track"]').click({ position: { x: 1, y: 1 } })
  assert.equal(await progress.inputValue(), '0')
  const uncontrolled = page.locator('#uncontrolled input')
  await uncontrolled.focus()
  await uncontrolled.press('End')
  assert.equal(await uncontrolled.inputValue(), '100')
  assert.equal(
    await page
      .locator('#uncontrolled [data-slot="slider-range"]')
      .evaluate((element) => element.style.width),
    '100%'
  )
  assert.deepEqual(
    await page
      .locator('#range [data-slot="slider-range"]')
      .evaluate((element) => ({
        start: element.style.insetInlineStart,
        size: element.style.width
      })),
    { start: '35%', size: '30%' }
  )
  assert.deepEqual(errors, [])
})
