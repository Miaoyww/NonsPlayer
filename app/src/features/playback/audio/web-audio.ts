import { browserOutput, releaseBrowserOutput, type createAudioOutput } from './audio-output'

export interface AudioSource {
  source: string
  startMs: number
  endMs: number | null
}
export interface AudioControl {
  serial: number
  source: string | null
  seekId: number
  positionMs: number
  startMs: number
  endMs: number | null
  playing: boolean
  volume: number
  fades: boolean
  crossfadeMs: number
  deviceId: string | null
  next: AudioSource | null
}
type FeedbackKind =
  | 'playing'
  | 'paused'
  | 'buffering'
  | 'progress'
  | 'ended'
  | 'error'
  | 'stopped'
  | 'transition'
  | 'transitionEnd'
  | 'preloadError'
  | 'deviceDefault'
export interface AudioFeedback {
  serial: number
  sequence: number
  kind: FeedbackKind
  positionMs: number
  durationMs: number
  nextSource?: string
  error?: string
}
type Output = ReturnType<typeof createAudioOutput>
type Voice = {
  spec: AudioSource
  audio: HTMLAudioElement
  gain: ReturnType<Output['attach']>
  detach: () => void
  seekId: number
  ended: boolean
  playing: boolean
  playPending: boolean
  pauseTimer?: ReturnType<typeof setTimeout>
}
const milliseconds = (seconds: number) =>
  Number.isFinite(seconds) ? Math.max(0, Math.round(seconds * 1000)) : 0

// IPC has one in-flight report and one coalesced pending report; audio never crosses IPC.
export function createWebAudio(
  send: (feedback: AudioFeedback) => Promise<unknown>,
  onError: (error: unknown) => void,
  createAudio: () => HTMLAudioElement = () => new Audio(),
  getOutput: () => Output = browserOutput
) {
  let control: AudioControl | undefined
  let active: Voice | undefined
  let nextVoice: Voice | undefined
  let outgoing: Voice | undefined
  let output: Output | undefined
  let sequence = 0
  let pending: AudioFeedback | undefined
  let sending = false
  let disposed = false
  let promoting: string | undefined
  let transitionStarting = false
  let transitionTimer: ReturnType<typeof setTimeout> | undefined
  let device = ''
  let deviceWork = Promise.resolve()
  async function flush() {
    if (sending) return
    sending = true
    try {
      while (pending && !disposed) {
        const value = pending
        pending = undefined
        try {
          await send(value)
        } catch (error) {
          onError(error)
        }
      }
    } finally {
      sending = false
    }
  }
  function position(voice: Voice | undefined, compensate = true) {
    if (!voice) return (control?.startMs ?? 0) + (control?.positionMs ?? 0)
    const latency =
      compensate && voice.playing && !voice.audio.seeking ? (output?.latency() ?? 0) : 0
    return Math.max(voice.spec.startMs, milliseconds(voice.audio.currentTime - latency))
  }
  function report(
    kind: FeedbackKind,
    error?: string,
    rawPosition?: number,
    nextSource?: string,
    rawDuration?: number
  ) {
    if (!control || disposed || (promoting && !['transition', 'stopped'].includes(kind))) return
    const value: AudioFeedback = {
      serial: control.serial,
      sequence: ++sequence,
      kind,
      positionMs: rawPosition ?? position(active),
      durationMs: rawDuration ?? milliseconds(active?.audio.duration ?? 0),
      error,
      nextSource
    }
    if (
      pending?.serial === value.serial &&
      ['ended', 'error', 'stopped', 'transition'].includes(pending.kind)
    )
      return
    pending = value
    void flush()
  }
  function destroy(voice: Voice | undefined) {
    if (!voice) return
    clearTimeout(voice.pauseTimer)
    voice.detach()
    voice.audio.pause()
    voice.audio.removeAttribute('src')
    voice.audio.load()
    voice.gain.release()
  }
  function finishOutgoing(notify = true) {
    clearTimeout(transitionTimer)
    transitionTimer = undefined
    destroy(outgoing)
    outgoing = undefined
    if (notify) {
      report('transitionEnd')
      if (control?.next && !nextVoice && !promoting && control.crossfadeMs > 0)
        nextVoice = makeVoice(control.next, true)
    }
  }
  function release() {
    finishOutgoing(false)
    destroy(nextVoice)
    destroy(active)
    active = nextVoice = undefined
    promoting = undefined
    transitionStarting = false
  }
  function finish() {
    if (!active || active.ended || promoting || transitionStarting) return
    active.ended = true
    active.audio.pause()
    report('ended')
  }
  function tryTransition() {
    const old = active,
      incoming = nextVoice,
      command = control
    if (
      !old ||
      !incoming ||
      !command?.playing ||
      !command.crossfadeMs ||
      promoting ||
      transitionStarting ||
      outgoing ||
      old.audio.seeking ||
      !old.playing ||
      incoming.audio.readyState < 3
    )
      return
    const end = old.spec.endMs ?? milliseconds(old.audio.duration)
    const remaining = end - milliseconds(old.audio.currentTime)
    if (remaining > command.crossfadeMs || remaining <= 0) return
    transitionStarting = true
    void (async () => {
      await output?.resume()
      await incoming.audio.play()
      if (
        disposed ||
        active !== old ||
        nextVoice !== incoming ||
        !control?.playing ||
        control.next?.source !== incoming.spec.source
      ) {
        if (active !== incoming) incoming.audio.pause()
        return
      }
      if (incoming.audio.ended) throw new Error('预加载音频已结束')
      active = incoming
      nextVoice = undefined
      outgoing = old
      incoming.playing = true
      incoming.seekId = control.seekId
      promoting = incoming.spec.source
      const seconds = Math.min(control.crossfadeMs, remaining) / 1000
      old.gain.ramp(0, seconds)
      incoming.gain.ramp(1, seconds)
      report('transition', undefined, position(incoming), incoming.spec.source)
      transitionTimer = setTimeout(() => finishOutgoing(), seconds * 1000 + 50)
    })()
      .catch((cause) => {
        if (nextVoice === incoming) {
          destroy(incoming)
          nextVoice = undefined
          report('preloadError', undefined, undefined, incoming.spec.source)
        }
        onError(cause)
      })
      .finally(() => {
        transitionStarting = false
        if (
          active === old &&
          (old.audio.ended ||
            milliseconds(old.audio.currentTime) >=
              (old.spec.endMs ?? milliseconds(old.audio.duration)))
        )
          finish()
      })
  }
  function makeVoice(spec: AudioSource, preload: boolean): Voice {
    output ??= getOutput()
    const audio = createAudio()
    audio.crossOrigin = 'anonymous'
    audio.preload = 'auto'
    audio.volume = 1
    const voice: Voice = {
      spec,
      audio,
      gain: output.attach(audio),
      detach: () => {},
      seekId: -1,
      ended: false,
      playing: false,
      playPending: false
    }
    voice.gain.ramp(preload || control?.fades ? 0 : 1, 0)
    const handlers: Record<string, () => void> = {
      loadedmetadata: () => {
        if (active === voice) synchronize()
        else if (nextVoice === voice) audio.currentTime = spec.startMs / 1000
      },
      canplay: () => {
        if (active === voice) synchronize()
        else tryTransition()
      },
      playing: () => {
        if (active !== voice) return
        if (!control?.playing) {
          audio.pause()
          return
        }
        voice.playing = true
        report('playing')
      },
      pause: () => {
        voice.playing = false
        if (active !== voice || voice.pauseTimer) return
        if (spec.endMs != null && milliseconds(audio.currentTime) >= spec.endMs) finish()
        else if (!control?.playing) report('paused')
      },
      waiting: () => {
        if (active === voice && control?.playing) report('buffering')
      },
      timeupdate: () => {
        if (active !== voice) return
        if (spec.endMs != null && milliseconds(audio.currentTime) >= spec.endMs) finish()
        else if (!voice.ended) {
          tryTransition()
          report(!audio.paused && !audio.seeking && audio.readyState >= 3 ? 'playing' : 'progress')
        }
      },
      seeked: () => {
        if (active === voice) {
          synchronize()
          report('progress', undefined, position(voice, false))
        }
      },
      ended: () => {
        if (active === voice) finish()
      },
      error: () => {
        if (active === voice)
          report('error', `浏览器不支持此音频或资源读取失败（${audio.error?.code ?? 0}）`)
        else if (nextVoice === voice) {
          nextVoice = undefined
          destroy(voice)
          report('preloadError', undefined, undefined, spec.source)
        }
      }
    }
    for (const [event, handler] of Object.entries(handlers)) audio.addEventListener(event, handler)
    voice.detach = () => {
      for (const [event, handler] of Object.entries(handlers))
        audio.removeEventListener(event, handler)
    }
    audio.src =
      spec.endMs == null
        ? spec.source
        : `${spec.source}#t=${spec.startMs / 1000},${spec.endMs / 1000}`
    audio.load()
    return voice
  }
  function synchronize() {
    const voice = active,
      command = control
    if (!voice || !command) return
    const audio = voice.audio
    if (!command.playing) {
      if (outgoing) {
        const seconds = command.fades ? 0.2 : 0
        outgoing.gain.ramp(0, seconds)
        clearTimeout(transitionTimer)
        transitionTimer = setTimeout(() => finishOutgoing(), seconds * 1000 + 50)
      }
      if (!voice.pauseTimer && !audio.paused) {
        const seconds = command.fades ? 0.2 : 0
        voice.gain.ramp(0, seconds)
        const pause = () => {
          voice.pauseTimer = undefined
          if (active !== voice || control?.playing) return
          audio.pause()
          voice.playing = false
          report('paused')
          void output?.suspend().catch(onError)
        }
        if (seconds) voice.pauseTimer = setTimeout(pause, 220)
        else pause()
      } else if (audio.paused) report('paused')
    } else {
      const reversing = voice.pauseTimer != null
      clearTimeout(voice.pauseTimer)
      voice.pauseTimer = undefined
      if (reversing) voice.gain.ramp(1, command.fades ? 0.2 : 0.02)
    }
    if (audio.readyState < 1) return
    if (voice.seekId !== command.seekId && !promoting) {
      voice.seekId = command.seekId
      voice.ended = false
      finishOutgoing(false)
      const target = (command.startMs + command.positionMs) / 1000
      try {
        audio.currentTime = Number.isFinite(audio.duration)
          ? Math.min(target, audio.duration)
          : target
      } catch {
        report('error', '浏览器无法定位此音频')
        return
      }
    }
    if (command.playing && audio.paused && !voice.ended && !voice.playPending) {
      voice.playPending = true
      void (async () => {
        await deviceWork
        await output?.resume()
        if (active !== voice || !control?.playing || disposed) return
        await audio.play()
        if (active !== voice || disposed) return
        if (!control?.playing) {
          audio.pause()
          return
        }
        voice.playing = true
        voice.gain.ramp(1, control.fades ? 0.2 : 0.02)
      })()
        .catch(() => {
          if (active === voice && control?.playing && control.serial === command.serial)
            report('error', '浏览器无法开始播放')
        })
        .finally(() => {
          voice.playPending = false
          if (
            active === voice &&
            control?.playing &&
            control.serial !== command.serial &&
            audio.paused
          )
            synchronize()
        })
    }
  }
  const mediaDevices = typeof navigator === 'undefined' ? undefined : navigator.mediaDevices
  const onDeviceChange = () => {
    if (!control?.source || !device || !output) return
    const expected = device,
      graph = output
    void mediaDevices
      ?.enumerateDevices()
      .then(async (devices) => {
        if (
          devices.some((d) => d.kind === 'audiooutput' && d.deviceId === expected) ||
          device !== expected ||
          output !== graph
        )
          return
        await graph.setDevice(null)
        device = ''
        report('deviceDefault')
      })
      .catch(onError)
  }
  mediaDevices?.addEventListener('devicechange', onDeviceChange)
  return {
    apply(next: AudioControl) {
      if (disposed || (control && next.serial <= control.serial)) return
      const previousPosition =
        active && active.audio.readyState >= 1
          ? position(active, false)
          : (control?.startMs ?? 0) + (control?.positionMs ?? 0)
      const previousDuration = milliseconds(active?.audio.duration ?? 0)
      const previousSource = active?.spec.source
      const wasCrossfading = (control?.crossfadeMs ?? 0) > 0
      control = next
      sequence = 0
      if (!next.source) {
        release()
        void output?.suspend().catch(onError)
        report('stopped', undefined, previousPosition, previousSource, previousDuration)
        return
      }
      if (promoting === next.source) {
        promoting = undefined
        if (active) active.seekId = next.seekId
      }
      const promotionPending = promoting && next.next?.source === promoting
      if (active?.spec.source !== next.source && !promotionPending) {
        release()
        active = makeVoice({ source: next.source, startMs: next.startMs, endMs: next.endMs }, false)
      }
      output ??= getOutput()
      output.setVolume(next.volume)
      if (device !== (next.deviceId ?? '')) {
        device = next.deviceId ?? ''
        const graph = output
        const requested = device
        deviceWork = graph
          .setDevice(next.deviceId)
          .catch(async (cause) => {
            if (output !== graph || device !== requested || disposed) return
            // A missing or revoked device recovers to system default.
            device = ''
            onError(cause)
            await graph.setDevice(null)
            report('deviceDefault')
          })
          .catch(onError)
      }
      if (nextVoice?.spec.source !== next.next?.source) {
        destroy(nextVoice)
        nextVoice = undefined
      }
      if (next.next && !nextVoice && !outgoing && !promoting && next.crossfadeMs > 0)
        nextVoice = makeVoice(next.next, true)
      if (!next.crossfadeMs) {
        finishOutgoing(false)
        if (wasCrossfading && next.playing) active?.gain.ramp(1, 0.02)
      }
      synchronize()
      if (promoting) report('transition', undefined, position(active), promoting)
    },
    dispose() {
      disposed = true
      mediaDevices?.removeEventListener('devicechange', onDeviceChange)
      pending = undefined
      release()
      output = undefined
      releaseBrowserOutput()
    }
  }
}
