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

function overlaps() {
  return [...document.querySelectorAll('.probe')].map((probe) => {
    return [...probe.querySelectorAll('[data-slot="morphing-text"]')].some((text) => {
      const letters = [...text.querySelectorAll('span[style*="inline-block"]')].map((letter) =>
        letter.getBoundingClientRect()
      )
      return letters.some((letter, index) => {
        const next = letters[index + 1]
        return next && letter.right > next.left + 0.5
      })
    })
  })
}

test('animating card width does not compress MorphingText glyphs', async (t) => {
  const root = fileURLToPath(new URL('../../', import.meta.url))
  const server = await createServer({
    root,
    server: { host: '127.0.0.1', port: 0, strictPort: false }
  })
  t.after(() => server.close())
  await server.listen()
  const browser = await launchBrowser()
  t.after(() => browser.close())
  const page = await browser.newPage({ viewport: { width: 1000, height: 400 } })
  await page.goto(`${server.resolvedUrls.local[0]}tests/browser/mv-lyric-card.html`)
  await page.locator('.card .text').first().waitFor()
  await page.waitForTimeout(400)

  for (const text of ['短い歌詞です', '吸って救わねんだってば']) {
    await page.evaluate((value) => window.setProbeText?.(value), text)
    await page.waitForTimeout(120)
    await page.screenshot({
      path: 'C:/Users/miaom/AppData/Local/Temp/mv-card-probe.png'
    })
    await page.waitForTimeout(380)
    const [surface, direct] = await page.evaluate(overlaps)
    assert.equal(surface, false, `surface implementation overlaps after ${text}`)
    assert.equal(direct, false, `direct implementation overlaps after ${text}`)
  }
})
