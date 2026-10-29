import { useEffect, useState } from 'react'
import {
  Button,
  usePluginConfig,
  usePluginFiles,
  usePluginNavigate,
  usePluginRoute,
  type ConfigSnapshot
} from '@nonsplayer/plugin-sdk'

export function SettingsPage() {
  const config = usePluginConfig()
  const files = usePluginFiles()
  const navigate = usePluginNavigate()
  const route = usePluginRoute()
  const [snapshot, setSnapshot] = useState<ConfigSnapshot>()
  const [text, setText] = useState('')
  const [error, setError] = useState('')
  const [result, setResult] = useState('')
  useEffect(() => {
    let disposed = false
    let cleanup: (() => void) | undefined
    const update = (value: ConfigSnapshot) => {
      if (!disposed) {
        setSnapshot(value)
        setText(String(value.values.greeting))
      }
    }
    void (async () => {
      const unlisten = await config.subscribe(update)
      if (disposed) {
        unlisten()
        return
      }
      cleanup = unlisten
      update(await config.getSnapshot())
    })().catch((e) => {
      if (!disposed) setError(String(e))
    })
    return () => {
      disposed = true
      cleanup?.()
    }
  }, [config])
  async function save() {
    if (!snapshot) return
    try {
      setSnapshot(await config.update({ greeting: text }, snapshot.revision))
      setError('')
    } catch (e) {
      setError(String(e))
      setSnapshot(await config.getSnapshot())
    }
  }
  async function binary() {
    try {
      const file = await files.open('data', 'sample.bin', 'readWrite')
      try {
        const bytes = new TextEncoder().encode(text)
        await file.truncate(0)
        await file.write(0, bytes)
        setResult(new TextDecoder().decode(await file.read(0)))
        setError('')
      } finally {
        await file.close()
      }
    } catch (e) {
      setError(String(e))
    }
  }
  return (
    <section style={{ border: '1px solid var(--border)', borderRadius: 12, padding: 20 }}>
      <h3>插件提供的配置页</h3>
      <p data-settings-route>{route.pathname}</p>
      <label>
        欢迎语
        <input
          aria-label="欢迎语"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onBlur={() => void save()}
          style={{
            display: 'block',
            width: '100%',
            margin: '12px 0',
            padding: 8,
            color: 'var(--foreground)',
            background: 'var(--background)',
            border: '1px solid var(--border)'
          }}
        />
      </label>
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        <Button onClick={() => navigate('/settings/advanced')}>打开子页面</Button>
        <Button onClick={() => void binary()}>写入并读取专属文件</Button>
      </div>
      {route.pathname.endsWith('/advanced') && (
        <Button onClick={() => navigate('/settings')}>返回配置首页</Button>
      )}
      {result && <p role="status">文件内容：{result}</p>}
      {error && <p role="alert">{error}</p>}
    </section>
  )
}
