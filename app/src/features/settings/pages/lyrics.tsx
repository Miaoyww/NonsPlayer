import type { CSSProperties } from 'react'
import { ErrorNotice } from '@/components/error-notice'
import { Slider } from '@/components/ui/slider'
import { Switch } from '@/components/ui/switch'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  defaultBackgroundSpeed,
  defaultLyricsFontScale,
  defaultLyricsOffsetMs,
  maxLyricsFontScale,
  minLyricsFontScale,
  useLyricsSettings
} from '@/features/lyrics/use-lyrics-settings'
import { useLyricSources } from '@/features/lyrics/use-lyric-sources'
import { DesktopLyricsSwitch } from '@/features/desktop-lyrics/desktop-lyrics-switch'
import { SettingsCard } from '@/features/settings/settings-card'
import { LyricFontWeightControl } from '@/features/settings/lyric-font-weight-control'
import { FontPicker } from '@/features/settings/font-picker'
import { useFontSettings } from '@/features/settings/use-font-settings'

export function LyricsPage() {
  const { fonts, setFont } = useFontSettings()
  const {
    backgroundSpeed,
    setBackgroundSpeed,
    lyricsOffsetMs,
    setLyricsOffsetMs,
    autoBlur,
    setAutoBlur,
    fontWeight,
    fontScale,
    setFontScale
  } = useLyricsSettings()
  const { sources, ready, busy, error, setSources } = useLyricSources()
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h2 className="text-xl font-bold">歌词设置</h2>
        <p className="mt-2 text-sm text-muted-foreground">调整歌词显示与来源。</p>
      </div>
      <section aria-labelledby="lyrics-display-heading" className="flex flex-col gap-8">
        <h3 id="lyrics-display-heading" className="text-base font-semibold">
          歌词显示
        </h3>
        <div
          className="lyrics-font-preview rounded-lg bg-muted/40 px-6 py-5"
          aria-label="歌词字体预览"
          style={
            {
              fontFamily: 'var(--nons-lyrics-font)',
              '--nons-lyrics-font-scale': String(fontScale)
            } as CSSProperties
          }
        >
          <h4 className="mb-5 text-xs font-medium text-muted-foreground">字体预览</h4>
          {[false, true].map((active) => (
            <div
              key={String(active)}
              className={`mb-5 last:mb-0 ${active ? 'text-foreground' : 'text-muted-foreground/60'}`}
            >
              <p className="leading-tight" style={{ fontWeight }}>
                我是一句歌词
              </p>
              <p className="mt-3" style={{ fontWeight, fontSize: '0.7em' }}>
                I'm the lyric
              </p>
              <p className="mt-1" style={{ fontWeight, fontSize: '0.7em' }}>
                wo shi yi ju ge ci
              </p>
            </div>
          ))}
        </div>
        <SettingsCard
          searchId="lyrics-font"
          title="歌词字体"
          description="用于歌词、翻译与发音，默认跟随应用字体。"
        >
          <FontPicker
            label="歌词字体"
            value={fonts.lyrics}
            onChange={(family) => setFont('lyrics', family)}
            defaultLabel="跟随应用字体"
          />
        </SettingsCard>
        <SettingsCard
          searchId="lyrics-font-weight"
          title="歌词字重"
          description="用于歌词、翻译与发音。"
        >
          <LyricFontWeightControl />
        </SettingsCard>
        <SettingsCard
          searchId="lyrics-font-size"
          title="字体大小"
          description="按当前字号缩放歌词、翻译与发音。"
        >
          <div className="flex w-60 flex-col gap-2">
            <div className="flex items-center gap-3">
              <Slider
                className="min-w-0 flex-1"
                aria-label="歌词字体大小"
                min={minLyricsFontScale}
                max={maxLyricsFontScale}
                step={0.05}
                value={fontScale}
                onValueChange={(value) => setFontScale(Number(value))}
              />
              <output
                className="w-12 text-right text-sm tabular-nums"
                aria-label="当前歌词字体大小"
              >
                {Math.round(fontScale * 100)}%
              </output>
            </div>
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>小</span>
              <span>大</span>
            </div>
            <Button
              variant="ghost"
              size="sm"
              className="self-end"
              onClick={() => setFontScale(defaultLyricsFontScale)}
            >
              恢复默认
            </Button>
          </div>
        </SettingsCard>
        <SettingsCard
          searchId="lyrics-auto-blur"
          title="歌词自动模糊"
          description="自动模糊远离当前播放位置的歌词，遵循系统减少动态效果设置。"
        >
          <Switch aria-label="歌词自动模糊" checked={autoBlur} onCheckedChange={setAutoBlur} />
        </SettingsCard>
        <SettingsCard
          searchId="lyrics-offset"
          title="歌词偏移"
          description="负数提前，正数延后；单曲偏移优先于全局设置。"
        >
          <div className="flex w-60 items-center gap-2">
            <Input
              className="w-28 text-right tabular-nums"
              aria-label="歌词偏移（毫秒）"
              type="number"
              min={-60000}
              max={60000}
              step={1}
              value={lyricsOffsetMs}
              onChange={(event) => setLyricsOffsetMs(event.currentTarget.valueAsNumber)}
            />
            <span className="text-sm text-muted-foreground">毫秒</span>
            <Button
              variant="ghost"
              size="sm"
              className="ml-auto"
              onClick={() => setLyricsOffsetMs(defaultLyricsOffsetMs)}
            >
              恢复默认
            </Button>
          </div>
        </SettingsCard>
        <SettingsCard
          searchId="lyrics-background-speed"
          title="背景流速"
          description="设为 0 停止流动，遵循系统减少动态效果设置。"
        >
          <div className="flex w-60 flex-col gap-2">
            <div className="flex items-center gap-3">
              <Slider
                className="min-w-0 flex-1"
                aria-label="背景流速"
                min={0}
                max={4}
                step={0.1}
                value={backgroundSpeed}
                onValueChange={(value) => setBackgroundSpeed(Number(value))}
              />
              <output className="w-8 text-right text-sm tabular-nums" aria-label="当前背景流速">
                {backgroundSpeed.toFixed(1)}
              </output>
            </div>
            <div className="flex justify-between text-xs text-muted-foreground">
              <span>静止</span>
              <span>快</span>
            </div>
            <Button
              variant="ghost"
              size="sm"
              className="self-end"
              onClick={() => setBackgroundSpeed(defaultBackgroundSpeed)}
            >
              恢复默认
            </Button>
          </div>
        </SettingsCard>
      </section>
      <section aria-labelledby="lyrics-desktop-heading" className="flex flex-col gap-8">
        <div>
          <h3 id="lyrics-desktop-heading" className="text-base font-semibold">
            桌面歌词
          </h3>
          <p className="mt-2 text-sm text-muted-foreground">
            在独立置顶窗口中显示当前歌词与翻译；锁定后鼠标可穿透，悬停歌词窗口可解锁。
          </p>
        </div>
        <SettingsCard
          searchId="lyrics-desktop"
          title="启用桌面歌词"
          description="关闭后桌面上不再显示歌词。"
        >
          <DesktopLyricsSwitch />
        </SettingsCard>
      </section>
      <section aria-labelledby="lyrics-sources-heading" className="flex flex-col gap-8">
        <div>
          <h3 id="lyrics-sources-heading" className="text-base font-semibold">
            来源
          </h3>
          <p className="mt-2 text-sm text-muted-foreground">
            流媒体歌曲始终在线获取歌词，依次尝试 AMLL DB、QQ 音乐、网易云音乐。
          </p>
        </div>
        <SettingsCard
          searchId="lyrics-amll"
          title="AMLL DB"
          description="优先获取 TTML 逐字歌词，失败时尝试下一来源。"
        >
          <label className="flex items-center gap-2 text-sm">
            <Switch
              checked={sources.amll}
              disabled={!ready || busy}
              onCheckedChange={(checked) => void setSources({ ...sources, amll: checked })}
            />
            从 AMLL DB 获取歌词
          </label>
        </SettingsCard>
        <SettingsCard
          searchId="lyrics-qq"
          title="QQ 音乐"
          description="匹配歌曲信息，未命中时使用网易云音乐。"
        >
          <label className="flex items-center gap-2 text-sm">
            <Switch
              checked={sources.qq}
              disabled={!ready || busy}
              onCheckedChange={(checked) => void setSources({ ...sources, qq: checked })}
            />
            从 QQ 音乐获取歌词
          </label>
        </SettingsCard>
        {error && <ErrorNotice error={error} />}
      </section>
    </div>
  )
}
