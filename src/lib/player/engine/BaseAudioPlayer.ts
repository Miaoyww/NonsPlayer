import {
  type IPlaybackEngine,
  type EngineCapabilities,
  type FadeCurve,
  type PlayOptions,
  type PauseOptions,
  type AudioEventType,
  type AudioErrorDetail,
  AUDIO_EVENTS,
  AudioErrorCode,
} from "./types";

// Re-export for convenience
export { AUDIO_EVENTS, AudioErrorCode };
export type { AudioEventType, AudioErrorDetail, AudioEventMap } from "./types";
import { AudioEffectManager } from "./AudioEffectManager";
import { getSharedAudioContext, getSharedMasterInput } from "./SharedAudioContext";

const SEEK_FADE_TIME = 0.05;

/**
 * Abstract base class for audio playback engines.
 *
 * Builds the Web Audio processing graph:
 *   Source (subclass) → InputNode → EffectManager → GainNode → MasterInput → Destination
 *
 * Provides fade-in/out, seek with crossfade, volume, and ReplayGain.
 */
export abstract class BaseAudioPlayer extends EventTarget implements IPlaybackEngine {
  protected audioCtx: AudioContext | null = null;
  protected gainNode: GainNode | null = null;
  protected inputNode: GainNode | null = null;
  protected effectManager: AudioEffectManager | null = null;

  protected compensatedLatency = 0;
  protected audioDelayCompensation = 0;

  protected isInitialized = false;
  protected volume = 1;
  protected replayGain = 1.0;

  private fadeTimer: ReturnType<typeof setTimeout> | null = null;

  abstract readonly capabilities: EngineCapabilities;

  // ── Initialization ──

  init() {
    if (this.isInitialized) return;

    try {
      this.audioCtx = getSharedAudioContext();
      this.inputNode = this.audioCtx.createGain();
      this.inputNode.gain.value = 1;
      this.gainNode = this.audioCtx.createGain();
      this.effectManager = new AudioEffectManager(this.audioCtx);

      // Chain: Input → EffectManager (EQ/Filters/Analyser) → GainNode → MasterInput
      const processedNode = this.effectManager.connect(this.inputNode);
      processedNode.connect(this.gainNode);
      this.gainNode.connect(getSharedMasterInput());

      // Latency compensation
      this.compensatedLatency =
        (this.audioCtx.outputLatency || 0) + (this.audioCtx.baseLatency || 0);

      this.gainNode.gain.value = this.volume;
      this.isInitialized = true;

      this.onGraphInitialized();
    } catch (e) {
      console.error("[BaseAudioPlayer] Failed to init AudioContext:", e);
    }
  }

  destroy(): void {
    if (this.gainNode) {
      this.gainNode.disconnect();
      this.gainNode = null;
    }
    if (this.inputNode) {
      this.inputNode.disconnect();
      this.inputNode = null;
    }
    if (this.effectManager) {
      this.effectManager.disconnect();
      this.effectManager = null;
    }
    this.audioCtx = null;
    this.isInitialized = false;
  }

  /** Subclasses connect their source node to this.inputNode here. */
  protected abstract onGraphInitialized(): void;

  // ── Playback control ──

  async play(url?: string, options: PlayOptions = {}): Promise<void> {
    this.cancelPendingPause();
    const shouldPlay = options.autoPlay ?? true;

    if (url) {
      await this.load(url);
    }

    if (!this.isInitialized) this.init();

    if (options.seek && options.seek > 0) {
      this.doSeek(options.seek);
    }

    if (!shouldPlay) return;

    if (this.audioCtx?.state === "suspended") {
      await this.audioCtx.resume();
    }

    const duration = options.fadeIn ? (options.fadeDuration ?? 0.5) : 0;
    if (duration > 0 && this.gainNode && this.audioCtx) {
      this.gainNode.gain.setValueAtTime(0, this.audioCtx.currentTime);
    }

    this.applyFadeTo(this.volume * this.replayGain, duration, options.fadeCurve);

    try {
      await this.doPlay();
    } catch (e) {
      console.error("[BaseAudioPlayer] Play failed:", e);
      throw e;
    }
  }

  async resume(options?: { fadeIn?: boolean; fadeDuration?: number }): Promise<void> {
    await this.play(undefined, options);
  }

  pause(options: PauseOptions = {}): void {
    this.cancelPendingPause();

    const duration = options.fadeOut ? (options.fadeDuration ?? 0.5) : 0;

    const performPause = async () => {
      await this.doPause();

      if (this.audioCtx && this.audioCtx.state === "running" && !options.keepContextRunning) {
        try {
          await this.audioCtx.suspend();
        } catch (e) {
          console.warn("[BaseAudioPlayer] Suspend AudioContext failed:", e);
        }
      }

      this.fadeTimer = null;
    };

    if (duration > 0) {
      this.applyFadeTo(0, duration, options.fadeCurve);
      this.fadeTimer = setTimeout(() => {
        performPause();
      }, duration * 1000);
    } else {
      performPause();
    }
  }

  async seek(time: number, immediate = false): Promise<void> {
    this.cancelPendingPause();

    if (this.paused) {
      this.doSeek(time);
      return;
    }

    if (!immediate) {
      this.applyFadeTo(0, SEEK_FADE_TIME);
      await new Promise((resolve) => setTimeout(resolve, SEEK_FADE_TIME * 1000));
    }

    await this.doSeek(time);

    this.applyFadeTo(
      this.volume * this.replayGain,
      immediate ? 0 : SEEK_FADE_TIME,
    );
  }

  stop(): void {
    this.cancelPendingPause();
    Promise.resolve(this.pause({ fadeOut: false })).catch(() => {});
    Promise.resolve(this.doSeek(0)).catch(() => {});
  }

  // ── Volume ──

  setVolume(value: number): void {
    this.volume = Math.max(0, Math.min(1, value));
    this.applyFadeTo(this.volume * this.replayGain, 0);
  }

  getVolume(): number {
    return this.volume;
  }

  rampVolumeTo(value: number, duration: number, curve?: FadeCurve): void {
    this.volume = Math.max(0, Math.min(1, value));
    this.applyFadeTo(this.volume * this.replayGain, duration, curve);
  }

  setReplayGain(gain: number): void {
    this.replayGain = gain;
    this.applyFadeTo(this.volume * this.replayGain, 0.1);
  }

  // ── Fade ──

  protected applyFadeTo(targetValue: number, duration: number, curve: FadeCurve = "linear"): void {
    if (!this.gainNode || !this.audioCtx) return;

    const currentTime = this.audioCtx.currentTime;
    this.gainNode.gain.cancelScheduledValues(currentTime);

    const currentValue = this.gainNode.gain.value;
    this.gainNode.gain.setValueAtTime(currentValue, currentTime);

    if (duration <= 0) {
      this.gainNode.gain.linearRampToValueAtTime(targetValue, currentTime + 0.02);
      return;
    }

    const safeStartTime = currentTime + 0.02;
    this.gainNode.gain.setValueAtTime(currentValue, safeStartTime);

    if (curve === "equalPower") {
      const steps = Math.max(2, Math.floor(duration * 60));
      const curveData = new Float32Array(steps);
      for (let i = 0; i < steps; i++) {
        const t = i / (steps - 1);
        curveData[i] = targetValue > currentValue
          ? currentValue + (targetValue - currentValue) * Math.sin((t * Math.PI) / 2)
          : targetValue + (currentValue - targetValue) * Math.cos((t * Math.PI) / 2);
      }
      this.gainNode.gain.setValueCurveAtTime(curveData, safeStartTime, duration);
    } else if (curve === "exponential") {
      const safeTarget = targetValue <= 0.001 ? 0.001 : targetValue;
      if (currentValue < 0.001) {
        this.gainNode.gain.linearRampToValueAtTime(targetValue, safeStartTime + duration);
      } else {
        this.gainNode.gain.exponentialRampToValueAtTime(safeTarget, safeStartTime + duration);
        if (targetValue === 0) {
          this.gainNode.gain.setValueAtTime(0, safeStartTime + duration);
        }
      }
    } else {
      this.gainNode.gain.linearRampToValueAtTime(targetValue, safeStartTime + duration);
    }
  }

  // ── Sink ──

  async setSinkId(deviceId: string): Promise<void> {
    if (deviceId === "default") return;
    if (this.audioCtx && typeof (this.audioCtx as any).setSinkId === "function") {
      try {
        await (this.audioCtx as any).setSinkId(deviceId);
        return;
      } catch (e) {
        console.warn("[BaseAudioPlayer] AudioContext setSinkId failed:", e);
      }
    }
    await this.doSetSinkId(deviceId);
  }

  // ── EQ / Filters / Spectrum ──

  setFilterGain(index: number, value: number): void {
    this.effectManager?.setFilterGain(index, value);
  }

  getFilterGains(): number[] {
    return this.effectManager?.getFilterGains() ?? [];
  }

  setHighPassFilter(frequency: number, rampTime = 0): void {
    this.effectManager?.setHighPassFilter(frequency, rampTime);
  }

  setHighPassQ(q: number): void {
    this.effectManager?.setHighPassQ(q);
  }

  setLowPassFilter(frequency: number, rampTime = 0): void {
    this.effectManager?.setLowPassFilter(frequency, rampTime);
  }

  setLowPassQ(q: number): void {
    this.effectManager?.setLowPassQ(q);
  }

  getFrequencyData(): Uint8Array {
    return this.effectManager?.getFrequencyData() ?? new Uint8Array(0);
  }

  getLowFrequencyVolume(): number {
    return this.effectManager?.getLowFrequencyVolume() ?? 0;
  }

  // ── Audio delay ──

  setAudioDelayCompensation(offset: number): void {
    this.audioDelayCompensation = offset;
  }

  // ── Event dispatch helpers ──

  protected dispatch(type: AudioEventType, detail?: AudioErrorDetail): void {
    if (detail !== undefined) {
      this.dispatchEvent(new CustomEvent(type, { detail }));
    } else {
      this.dispatchEvent(new CustomEvent(type, { detail: undefined }));
    }
  }

  // ── Pending pause guard ──

  protected cancelPendingPause(): void {
    if (this.fadeTimer) {
      clearTimeout(this.fadeTimer);
      this.fadeTimer = null;
    }
  }

  // ── Abstract methods ──

  abstract load(url: string): Promise<void>;
  protected abstract doPlay(): Promise<void>;
  protected abstract doPause(): void | Promise<void>;
  protected abstract doSeek(time: number): void | Promise<void>;
  abstract setRate(value: number): void;
  abstract getRate(): number;
  protected abstract doSetSinkId(deviceId: string): Promise<void>;
  abstract get src(): string;
  abstract get duration(): number;
  abstract get currentTime(): number;
  abstract get paused(): boolean;
  abstract getErrorCode(): number;
}
