import { Check, Clapperboard, FolderOpen, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { LocalMv } from './local-mv'

export function MvPanel({
  selection,
  linked,
  busy,
  onChoose,
  onApply,
  onCancel
}: {
  selection?: LocalMv
  linked: boolean
  busy: boolean
  onChoose: () => void
  onApply: () => void
  onCancel: () => void
}) {
  return (
    <section className="mv-panel" aria-label="本地 MV">
      <header className="mv-panel-header">
        <h2>本地 MV</h2>
        <p>为当前歌曲选择本地视频，确认后再进入 MV 画面。</p>
      </header>
      <div className="mv-panel-card">
        <div className="mv-panel-selection">
          <span className="mv-panel-icon" aria-hidden="true">
            <Clapperboard />
          </span>
          <div className="mv-panel-file">
            {selection ? (
              <>
                <strong>{selection.name}</strong>
                <span title={selection.path}>{selection.path}</span>
                <small data-state={selection.available ? (linked ? 'linked' : 'ready') : 'missing'}>
                  {selection.available
                    ? linked
                      ? '已关联'
                      : '尚未应用'
                    : '已关联的视频不可用，请重新选择'}
                </small>
              </>
            ) : (
              <>
                <strong>尚未选择视频</strong>
                <span>支持 MP4、M4V、WebM</span>
              </>
            )}
          </div>
        </div>
        <div className="mv-panel-actions">
          <Button type="button" variant="outline" disabled={busy} onClick={onChoose}>
            <FolderOpen aria-hidden="true" />
            {selection ? '更换视频' : '选择视频'}
          </Button>
          <Button
            type="button"
            variant="secondary"
            disabled={busy || !selection?.available}
            onClick={onApply}
          >
            <Check aria-hidden="true" />
            {busy ? '处理中…' : '应用'}
          </Button>
          <Button type="button" variant="ghost" disabled={busy} onClick={onCancel}>
            <X aria-hidden="true" />
            返回
          </Button>
        </div>
      </div>
      <p className="mv-panel-hint">MV 静音播放，并跟随当前歌曲进度对齐。</p>
    </section>
  )
}
