import { createRoot } from 'react-dom/client'
import { ExtensionStore } from '../../src/features/settings/extensions/extension-store'
import '../../src/styles/globals.css'

const host = globalThis as unknown as {
  isTauri: boolean
  __TAURI_INTERNALS__: {
    invoke: (command: string, args: Record<string, unknown>) => Promise<unknown>
  }
}
host.isTauri = true
host.__TAURI_INTERNALS__ = {
  async invoke(command, args) {
    if (command === 'extension_store_index')
      return {
        source: args?.source ?? 'cdn',
        url:
          args?.source === 'github'
            ? 'https://raw.githubusercontent.com/NonsPlayer/plugins/meta/index.json'
            : 'https://nons-plugn.miaoyww.top/index.json',
        generatedAt: '2026-10-30T12:00:00Z',
        plugins: Array.from({ length: 24 }, (_, index) => ({
          id: `example-${index}`,
          name: `音乐工具 ${index + 1}`,
          kind: 'plugin',
          version: '2.0.0',
          description: '播放器扩展：查看歌曲信息并管理本地文件。',
          authors: [{ name: 'NonsPlayer' }],
          license: 'GPL-3.0-only',
          repository: 'https://github.com/NonsPlayer/example',
          labels: ['tool'],
          permissions: ['ui', 'files:selected'],
          fileRoots: ['*'],
          total_download_count: [0, 999, 1000, 1250][index % 5],
          compatible: true,
          bundled: false,
          updateAvailable: false,
          artifact: {
            url: 'https://github.com/NonsPlayer/example/releases/download/v2.0.0/example.zip',
            size: 1024,
            sha256: '0'.repeat(64)
          }
        })),
        adapters: []
      }
    if (command === 'extension_store_prepare')
      return {
        token: 'prepared',
        kind: 'plugin',
        id: args.id,
        name: '音乐工具 1',
        version: '2.0.0',
        permissions: ['运行受信任的界面代码', '读取、创建、修改和删除任意文件'],
        warnings: Array.from(
          { length: 18 },
          () => '前端与播放器共享界面环境，请仅授权你信任的插件。'
        )
      }
    return null
  }
}

createRoot(document.getElementById('root')!).render(
  <main className="mx-auto max-w-3xl space-y-4 p-4 pb-24">
    <h1 className="text-xl font-bold">插件设置</h1>
    <ExtensionStore kind="plugin" onInstalled={() => undefined} />
    <div role="contentinfo">列表末尾</div>
  </main>
)
