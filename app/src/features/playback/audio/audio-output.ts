type SinkContext = AudioContext & { setSinkId?: (id: string) => Promise<void> }

// One context/output per executor; per-track gains allow two bounded, overlapping voices.
export function createAudioOutput(
  createContext = () => new AudioContext({ latencyHint: 'playback' })
) {
  const context = createContext() as SinkContext
  const volume = context.createGain()
  const limiter = context.createDynamicsCompressor()
  volume.connect(limiter)
  limiter.threshold.value = -1
  limiter.knee.value = 0
  limiter.ratio.value = 20
  limiter.attack.value = 0.003
  limiter.release.value = 0.25
  limiter.connect(context.destination)
  let fallback: HTMLAudioElement | undefined
  let destination: MediaStreamAudioDestinationNode | undefined
  let sinkId = ''
  let sinkQueue = Promise.resolve()
  let transportQueue = Promise.resolve()
  let wantsRunning = false
  let disposed = false
  let volumeInitialized = false
  function transport(running: boolean) {
    wantsRunning = running
    const result = transportQueue.then(async () => {
      if (disposed) return
      if (wantsRunning) {
        if (context.state === 'suspended') await context.resume()
        if (!disposed && wantsRunning && fallback?.paused) await fallback.play()
      } else {
        if (context.state === 'running') await context.suspend()
        fallback?.pause()
      }
    })
    transportQueue = result.catch(() => {})
    return result
  }
  return {
    context,
    get supportsDevices() {
      return (
        typeof context.setSinkId === 'function' ||
        typeof HTMLMediaElement.prototype.setSinkId === 'function'
      )
    },
    setVolume(value: number) {
      const now = context.currentTime
      volume.gain.cancelScheduledValues(now)
      if (!volumeInitialized || value === 0) volume.gain.setValueAtTime(value, now)
      else volume.gain.setTargetAtTime(value, now, 0.01)
      volumeInitialized = true
    },
    attach(audio: HTMLAudioElement) {
      const source = context.createMediaElementSource(audio)
      const gain = context.createGain()
      source.connect(gain)
      gain.connect(volume)
      let ramp = { from: 1, to: 1, start: 0, end: 0 }
      return {
        ramp(value: number, seconds: number) {
          const now = context.currentTime
          const fraction =
            ramp.end > ramp.start
              ? Math.min(1, Math.max(0, (now - ramp.start) / (ramp.end - ramp.start)))
              : 1
          const current = ramp.from + (ramp.to - ramp.from) * fraction
          gain.gain.cancelScheduledValues(now)
          gain.gain.setValueAtTime(current, now)
          gain.gain.linearRampToValueAtTime(value, now + seconds)
          ramp = { from: current, to: value, start: now, end: now + seconds }
        },
        release() {
          source.disconnect()
          gain.disconnect()
        }
      }
    },
    resume() {
      return transport(true)
    },
    suspend() {
      return transport(false)
    },
    setDevice(id: string | null) {
      // Serialize changes so an older asynchronous selection cannot win.
      const requested = id ?? ''
      const result = sinkQueue.then(async () => {
        if (disposed) throw new Error('音频输出已关闭')
        if (requested === sinkId) return
        let contextError: unknown
        if (!fallback && context.setSinkId) {
          try {
            await context.setSinkId(requested)
            if (!disposed) sinkId = requested
            return
          } catch (cause) {
            contextError = cause
          }
        }
        if (typeof HTMLMediaElement.prototype.setSinkId !== 'function')
          throw contextError ?? new Error('系统 WebView 不支持输出设备选择')
        if (fallback) {
          await fallback.setSinkId(requested)
          if (!disposed && wantsRunning) await fallback.play()
        } else {
          const candidate = new Audio()
          const streamOutput = context.createMediaStreamDestination()
          candidate.srcObject = streamOutput.stream
          try {
            await candidate.setSinkId(requested)
            if (disposed) throw new Error('音频输出已关闭')
            if (wantsRunning) await candidate.play()
            if (disposed) throw new Error('音频输出已关闭')
            // Commit routing only after selection succeeds, preserving output on failure.
            limiter.disconnect()
            limiter.connect(streamOutput)
            fallback = candidate
            destination = streamOutput
          } catch (cause) {
            candidate.pause()
            candidate.srcObject = null
            streamOutput.disconnect()
            throw cause
          }
        }
        sinkId = requested
      })
      sinkQueue = result.catch(() => {})
      return result
    },
    latency() {
      return Math.max(0, context.baseLatency || 0) + Math.max(0, context.outputLatency || 0)
    },
    dispose() {
      disposed = true
      wantsRunning = false
      fallback?.pause()
      if (fallback) fallback.srcObject = null
      volume.disconnect()
      limiter.disconnect()
      destination?.disconnect()
      void context.close().catch(() => {})
    }
  }
}

let output: ReturnType<typeof createAudioOutput> | undefined
export function browserOutput() {
  return (output ??= createAudioOutput())
}
export function releaseBrowserOutput() {
  output?.dispose()
  output = undefined
}
export async function browserOutputDevices() {
  if (!browserOutput().supportsDevices) return []
  const devices = await navigator.mediaDevices.enumerateDevices()
  return devices
    .filter(
      (device) => device.kind === 'audiooutput' && device.deviceId && device.deviceId !== 'default'
    )
    .map((device, i) => ({ id: device.deviceId, name: device.label || `音频输出 ${i + 1}` }))
}
export async function selectBrowserOutput(id: string | null) {
  await browserOutput().setDevice(id)
}
