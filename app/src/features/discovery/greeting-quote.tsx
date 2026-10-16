import { reportCaughtError } from '@/lib/diagnostics'
import { useEffect, useState } from 'react'
import { nativeCall } from '@/lib/player'
import { TypingAnimation } from '@/components/magicui/typing-animation'

const fallback = '由此开启好心情 ～'

export function GreetingQuote() {
  const [quote, setQuote] = useState<string>()
  useEffect(() => {
    let disposed = false
    void nativeCall<string>('discovery_hitokoto')
      .then((value) => {
        if (!disposed) setQuote(value.trim() || fallback)
      })
      .catch((cause) => {
        reportCaughtError('app/src/features/discovery/greeting-quote.tsx', cause)

        if (!disposed) setQuote(fallback)
      })
    return () => {
      disposed = true
    }
  }, [])
  const text = quote ?? fallback
  return (
    <p className="discover-quote">
      <span className="invisible" aria-hidden="true">
        {text}
      </span>
      <span className="sr-only">{text}</span>
      {quote ? (
        <TypingAnimation
          key={quote}
          duration={85}
          className="discover-quote-typing"
          aria-hidden="true"
        >
          {quote}
        </TypingAnimation>
      ) : (
        <span className="discover-quote-typing" aria-hidden="true">
          {text}
        </span>
      )}
    </p>
  )
}
