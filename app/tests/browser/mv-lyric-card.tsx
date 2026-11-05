import { MorphingText } from '@animate-ui/react/primitives/texts/morphing'
import { motion } from 'motion/react'
import { useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'

const initial = '吸って救わねんだってば'

function Probe({ surface, text }: { surface: boolean; text: string }) {
  if (surface)
    return (
      <div className="card">
        <motion.div className="surface" layout="size" transition={{ layout: { duration: 0.28 } }} />
        <MorphingText className="text" text={text} />
        <MorphingText className="translation" text="她想被吃掉救吗" />
      </div>
    )
  return (
    <motion.div className="card" layout="size" transition={{ layout: { duration: 0.28 } }}>
      <MorphingText className="text" text={text} />
      <MorphingText className="translation" text="她想被吃掉救吗" />
    </motion.div>
  )
}

function App() {
  const [text, setText] = useState(initial)
  useEffect(() => {
    window.setProbeText = setText
    return () => {
      window.setProbeText = undefined
    }
  }, [])
  return (
    <>
      <div className="probe">
        <strong>surface</strong>
        <Probe surface text={text} />
      </div>
      <div className="probe">
        <strong>direct</strong>
        <Probe surface={false} text={text} />
      </div>
    </>
  )
}

declare global {
  interface Window {
    setProbeText?: (text: string) => void
  }
}

createRoot(document.getElementById('root')!).render(<App />)
