import { StrictMode, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { createRoot } from 'react-dom/client'
import { LyricPlayer, type LyricPlayerRef } from '@applemusic-like-lyrics/react'
import type { LyricLine } from '@applemusic-like-lyrics/core'
import '@applemusic-like-lyrics/core/style.css'

type Spec = {
  start: number
  end: number
  text: string
  translation: string
}

const specs: Spec[] = [
  {
    start: 0,
    end: 3600,
    text: '晚风穿过安静的街道',
    translation: 'The evening breeze drifts down the quiet street'
  },
  {
    start: 3800,
    end: 7600,
    text: '把路灯吹成一片海',
    translation: 'Turning streetlights into an ocean'
  },
  {
    start: 7800,
    end: 11800,
    text: '我在人群中慢慢走',
    translation: 'I walk slowly through the crowd'
  },
  {
    start: 12000,
    end: 16000,
    text: '听见你名字的回响',
    translation: 'Hearing the echo of your name'
  },
  {
    start: 16200,
    end: 20200,
    text: '如果时间可以倒流',
    translation: 'If time could run backwards'
  },
  {
    start: 20400,
    end: 24600,
    text: '我想停在这一秒',
    translation: 'I would stay inside this second'
  },
  {
    start: 24800,
    end: 28800,
    text: '让所有没说完的话',
    translation: 'Let every unfinished sentence'
  },
  {
    start: 29000,
    end: 33600,
    text: '都变成今晚的星光',
    translation: 'Become the starlight tonight'
  }
]

const totalMs = 35000

function toLyricLine(spec: Spec, showTranslation: boolean): LyricLine {
  const chars = [...spec.text]
  const step = (spec.end - spec.start) / chars.length
  return {
    startTime: spec.start,
    endTime: spec.end,
    words: chars.map((word, index) => ({
      startTime: Math.round(spec.start + index * step),
      endTime: Math.round(spec.start + (index + 1) * step),
      word
    })),
    translatedLyric: showTranslation ? spec.translation : '',
    romanLyric: '',
    isBG: false,
    isDuet: false
  }
}

function format(ms: number) {
  const seconds = Math.max(0, Math.floor(ms / 1000))
  return `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`
}

function Preview() {
  const [playing, setPlaying] = useState(true)
  const [showTranslation, setShowTranslation] = useState(true)
  const [hidePassedLines, setHidePassedLines] = useState(false)
  const [showOutline, setShowOutline] = useState(true)
  const [checker, setChecker] = useState(false)
  const [fontSize, setFontSize] = useState(40)
  const [fadeWidth, setFadeWidth] = useState(0.5)
  const [displayTime, setDisplayTime] = useState(0)
  const playerRef = useRef<LyricPlayerRef | null>(null)
  const time = useRef(0)
  const playingRef = useRef(playing)
  playingRef.current = playing
  const lines = useMemo(
    () => specs.map((spec) => toLyricLine(spec, showTranslation)),
    [showTranslation]
  )

  useEffect(() => {
    let frame = 0
    let previous = performance.now()
    let lastPaint = previous
    const tick = (now: number) => {
      const delta = Math.min(50, now - previous)
      previous = now
      if (playingRef.current) {
        time.current += delta
        if (time.current >= totalMs) time.current = 0
      }
      const player = playerRef.current?.lyricPlayer
      player?.setCurrentTime(time.current)
      player?.update(delta)
      if (now - lastPaint > 100) {
        lastPaint = now
        setDisplayTime(time.current)
      }
      frame = requestAnimationFrame(tick)
    }
    frame = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(frame)
  }, [])

  const restart = () => {
    time.current = 0
    playerRef.current?.lyricPlayer?.setCurrentTime(0, true)
    setDisplayTime(0)
    setPlaying(true)
  }

  const height = fontSize * (showTranslation ? 2.9 : 2.1)

  return (
    <div className={`stage${checker ? ' checker' : ''}`}>
      <div className="wallpaper-meta">Nons · 桌面歌词效果预览</div>
      <div
        className={`lyric-window${showOutline ? ' outline' : ''}`}
        style={
          {
            height,
            '--preview-font-size': `${fontSize}px`
          } as CSSProperties
        }
      >
        <LyricPlayer
          ref={playerRef}
          className="preview-player"
          lyricLines={lines}
          playing={playing}
          disabled
          alignAnchor="center"
          alignPosition={0.5}
          enableBlur={false}
          enableScale={false}
          hidePassedLines={hidePassedLines}
          wordFadeWidth={fadeWidth}
          style={{ width: '100%', height: '100%' }}
        />
      </div>
      <p className="hint">
        这是浏览器里的渲染预览。真实桌面歌词窗口会无边框、透明、始终置顶，并可鼠标穿透。
        切换歌词时可以观察单行窗口如何裁掉相邻行。
      </p>
      <div className="panel">
        <h1>单行歌词预览</h1>
        <p>AMLL 本身没有单行开关，靠一行高容器裁剪实现。</p>
        <div className="buttons">
          <button
            className={playing ? '' : 'primary'}
            onClick={() => setPlaying((value) => !value)}
          >
            {playing ? '暂停' : '播放'}
          </button>
          <button onClick={restart}>重新开始</button>
        </div>
        <div className="row">
          <label htmlFor="translation">显示翻译</label>
          <input
            id="translation"
            type="checkbox"
            checked={showTranslation}
            onChange={(event) => setShowTranslation(event.target.checked)}
          />
        </div>
        <div className="row">
          <label htmlFor="passed">隐藏已唱行</label>
          <input
            id="passed"
            type="checkbox"
            checked={hidePassedLines}
            onChange={(event) => setHidePassedLines(event.target.checked)}
          />
        </div>
        <div className="row">
          <label htmlFor="outline">显示裁剪框</label>
          <input
            id="outline"
            type="checkbox"
            checked={showOutline}
            onChange={(event) => setShowOutline(event.target.checked)}
          />
        </div>
        <div className="row">
          <label htmlFor="checker">透明棋盘格</label>
          <input
            id="checker"
            type="checkbox"
            checked={checker}
            onChange={(event) => setChecker(event.target.checked)}
          />
        </div>
        <div className="row">
          <label htmlFor="size">字号</label>
          <input
            id="size"
            type="range"
            min="24"
            max="56"
            value={fontSize}
            onChange={(event) => setFontSize(Number(event.target.value))}
          />
          <output>{fontSize}px</output>
        </div>
        <div className="row">
          <label htmlFor="fade">渐变宽度</label>
          <input
            id="fade"
            type="range"
            min="0.0001"
            max="1"
            step="0.05"
            value={fadeWidth}
            onChange={(event) => setFadeWidth(Number(event.target.value))}
          />
          <output>{fadeWidth.toFixed(2)}</output>
        </div>
        <div className="progress">
          <div style={{ width: `${(displayTime / totalMs) * 100}%` }} />
        </div>
        <div className="time">
          <span>{format(displayTime)}</span>
          <span>{format(totalMs)}</span>
        </div>
      </div>
    </div>
  )
}

createRoot(document.getElementById('root') as HTMLElement).render(
  <StrictMode>
    <Preview />
  </StrictMode>
)
