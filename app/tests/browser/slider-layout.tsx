import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import { PlayerSlider } from '../../src/features/playback/player-slider'
import { Slider } from '../../src/components/ui/slider'
import '../../src/styles/globals.css'

function SliderLayout() {
  const [value, setValue] = useState(50)
  return (
    <div className="now-playing" style={{ padding: 24 }}>
      <label>
        测试数值
        <input
          id="value"
          type="number"
          value={value}
          onChange={(event) => setValue(Number(event.target.value))}
        />
      </label>
      <div
        id="details"
        className="now-playing-details flex min-h-0 min-w-0 flex-col overflow-y-auto"
        style={{ maxWidth: 'none', height: 320 }}
      >
        <div style={{ height: 180, flexShrink: 0 }}>歌曲信息</div>
        <div className="now-playing-controls flex shrink-0 flex-col">
          <div id="progress" className="playback-timeline playback-timeline-below">
            <span className="timeline-elapsed">4:53</span>
            <PlayerSlider
              className="timeline-slider"
              aria-label="播放进度"
              value={value}
              onValueChange={(next) => setValue(Number(next))}
            />
            <span className="timeline-duration">-0:02</span>
          </div>
          <div id="volume-row" className="flex items-center gap-3">
            <span>低</span>
            <PlayerSlider
              id="volume"
              className="min-w-0 flex-1"
              aria-label="音量"
              min={0}
              max={1}
              step={0.01}
              value={value / 100}
            />
            <span>高</span>
          </div>
          <div style={{ height: 180 }}>播放控制</div>
        </div>
      </div>
      <div id="vertical-container" style={{ height: 160, width: 32, overflow: 'auto', margin: 24 }}>
        <Slider
          id="vertical"
          orientation="vertical"
          min={0}
          max={1}
          step={0.01}
          value={value / 100}
          aria-label="纵向音量"
        />
      </div>
      <Slider id="range" defaultValue={[20, 80]} min={-50} max={150} aria-label="范围" />
      <Slider id="uncontrolled" defaultValue={25} aria-label="非受控数值" />
    </div>
  )
}

createRoot(document.getElementById('root')!).render(<SliderLayout />)
