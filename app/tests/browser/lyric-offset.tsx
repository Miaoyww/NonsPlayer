import { useState } from 'react'
import { createRoot } from 'react-dom/client'
import { LyricOffsetControl } from '../../src/features/lyrics/lyric-offset-control'
import '../../src/styles/globals.css'

function Probe() {
  const [offsetMs, setOffsetMs] = useState(0)
  return (
    <main
      className="nons-lyrics now-playing"
      style={{ position: 'relative', height: '100vh', background: '#302448', color: 'white' }}
    >
      <div style={{ position: 'absolute', top: 8, left: 40, zIndex: 20 }}>
        <button onClick={() => setOffsetMs(60000)}>上限</button>
        <button onClick={() => setOffsetMs(-60000)}>下限</button>
      </div>
      <div className="now-playing-content">
        <div className="now-playing-layout has-lyrics relative min-h-0 flex-1">
          <div className="now-playing-details" aria-label="封面区域">
            音乐封面
          </div>
          <div className="now-playing-panel-region" data-panel-page="lyrics">
            <div className="now-playing-lyrics min-h-0 min-w-0 overflow-hidden">
              <div
                className="lyric-renderer h-full w-full"
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  justifyContent: 'center',
                  gap: 24
                }}
              >
                <p style={{ fontSize: 32, fontWeight: 600 }}>
                  The more breaks I get, the better I stay
                </p>
                <p style={{ fontSize: 32, fontWeight: 600 }}>Life goes on</p>
                <p style={{ fontSize: 32, fontWeight: 600 }}>So I need a change</p>
              </div>
            </div>
          </div>
        </div>
      </div>
      <LyricOffsetControl offsetMs={offsetMs} onChange={setOffsetMs} />
    </main>
  )
}

createRoot(document.getElementById('root')!).render(<Probe />)
