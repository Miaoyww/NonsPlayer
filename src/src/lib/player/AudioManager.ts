import { AudioElementPlayer } from "./engine/AudioElementPlayer";
import { AUDIO_EVENTS } from "./engine/BaseAudioPlayer";
import type {
  IPlaybackEngine,
  EngineCapabilities,
  FadeCurve,
  PlayOptions,
  PauseOptions,
} from "./engine/types";

/**
 * AudioManager — engine facade.
 *
 * Selects the playback engine and provides a unified interface for
 * play, pause, crossfade, seek, volume, rate, EQ, and spectrum.
 * All events from the underlying engine are forwarded.
 *
 * Currently only AudioElementPlayer is supported. FFmpeg WASM and
 * MPV engines can be added later when needed.
 */
class AudioManager extends EventTarget implements IPlaybackEngine {
  private engine: IPlaybackEngine;
  private pendingEngine: IPlaybackEngine | null = null;
  private pendingSwitchTimer: ReturnType<typeof setTimeout> | null = null;
  private cleanupListeners: (() => void) | null = null;
  private isCrossfading = false;
  private _masterVolume = 1.0;

  readonly engineType: "element" | "ffmpeg" | "mpv";
  readonly capabilities: EngineCapabilities;

  constructor() {
    super();

    // Default to HTML5 Audio element engine
    this.engine = new AudioElementPlayer();
    this.engineType = "element";
    this.capabilities = this.engine.capabilities;
    this.bindEngineEvents();
  }

  // ── Event forwarding ──

  private bindEngineEvents(): void {
    if (this.cleanupListeners) {
      this.cleanupListeners();
    }

    const events = Object.values(AUDIO_EVENTS);
    const handlers: Map<string, EventListener> = new Map();

    for (const eventType of events) {
      const handler = (e: Event) => {
        // During crossfade, suppress old engine's pause/ended/error
        if (
          this.isCrossfading &&
          (eventType === "pause" || eventType === "ended" || eventType === "error")
        ) {
          return;
        }
        const detail = (e as CustomEvent).detail;
        this.dispatchEvent(new CustomEvent(eventType, { detail }));
      };
      handlers.set(eventType, handler);
      this.engine.addEventListener(eventType, handler);
    }

    this.cleanupListeners = () => {
      handlers.forEach((handler, eventType) => {
        this.engine.removeEventListener(eventType, handler);
      });
    };
  }

  // ── IPlaybackEngine implementation ──

  init(): void {
    this.engine.init();
  }

  destroy(): void {
    this.clearPendingSwitch();
    if (this.cleanupListeners) {
      this.cleanupListeners();
      this.cleanupListeners = null;
    }
    this.engine.destroy();
  }

  async play(url?: string, options?: PlayOptions): Promise<void> {
    await this.engine.play(url, options);
  }

  // ── Crossfade ──

  async crossfadeTo(
    url: string,
    options: {
      duration: number;
      seek?: number;
      autoPlay?: boolean;
      uiSwitchDelay?: number;
      onSwitch?: () => void;
      mixType?: "default" | "bassSwap";
      rate?: number;
      replayGain?: number;
      fadeCurve?: FadeCurve;
    },
  ): Promise<void> {
    console.log(
      `[AudioManager] Crossfade (duration: ${options.duration}s, type: ${options.mixType})`,
    );

    this.clearPendingSwitch();
    this.isCrossfading = true;

    const newEngine = new AudioElementPlayer();
    newEngine.init();
    this.pendingEngine = newEngine;

    newEngine.setVolume(0);

    if (options.rate !== undefined) {
      newEngine.setRate(options.rate);
    }
    if (options.replayGain !== undefined) {
      newEngine.setReplayGain?.(options.replayGain);
    }

    // Bass-swap: exchange low frequencies between old and new engine
    if (options.mixType === "bassSwap") {
      this.engine.setHighPassQ?.(1.0);
      newEngine.setHighPassQ?.(1.0);
      newEngine.setHighPassFilter?.(400, 0);
    }

    const fadeCurve = options.fadeCurve ?? "equalPower";

    await newEngine.play(url, {
      autoPlay: true,
      seek: options.seek,
      fadeIn: false,
    });

    newEngine.rampVolumeTo?.(this._masterVolume, options.duration, fadeCurve);

    // Bass-swap filter automation
    if (options.mixType === "bassSwap") {
      const mid = options.duration * 0.5;
      const release = Math.min(0.6, options.duration * 0.25);
      const ctx = new AudioContext(); // temp context for timing — just use setTimeout as fallback
      ctx.close();

      this.engine.setHighPassFilter?.(400, mid);

      if (newEngine.setHighPassFilter) {
        newEngine.setHighPassFilter(400, 0);
        setTimeout(() => {
          newEngine.setHighPassFilter?.(10, release);
        }, mid * 1000);
      }
    }

    // Fade out old engine
    const oldEngine = this.engine;
    oldEngine.pause({
      fadeOut: true,
      fadeDuration: options.duration,
      fadeCurve,
      keepContextRunning: true,
    });

    const commitSwitch = () => {
      console.log("[AudioManager] Committing crossfade switch");
      if (this.cleanupListeners) {
        this.cleanupListeners();
        this.cleanupListeners = null;
      }

      this.engine = newEngine;
      this.pendingEngine = null;
      this.isCrossfading = false;
      this.bindEngineEvents();

      try {
        options.onSwitch?.();
      } catch (e) {
        console.error("[AudioManager] onSwitch callback failed:", e);
      }

      this.dispatchEvent(new CustomEvent(AUDIO_EVENTS.TIME_UPDATE, { detail: undefined }));
      this.dispatchEvent(new CustomEvent(AUDIO_EVENTS.PLAY, { detail: undefined }));

      if (options.mixType !== "bassSwap") {
        this.engine.setHighPassFilter?.(0, 0);
      }
    };

    const switchDelay = options.uiSwitchDelay ?? 0;
    if (switchDelay > 0) {
      this.pendingSwitchTimer = setTimeout(() => {
        this.pendingSwitchTimer = null;
        commitSwitch();
      }, switchDelay * 1000);
    } else {
      commitSwitch();
    }

    setTimeout(() => oldEngine.destroy(), options.duration * 1000 + 1000);
  }

  async resume(options?: { fadeIn?: boolean; fadeDuration?: number }): Promise<void> {
    await this.engine.resume(options);
  }

  pause(options?: PauseOptions): void {
    this.engine.pause(options);
  }

  stop(): void {
    this.clearPendingSwitch();
    this.engine.stop();
  }

  seek(time: number): void {
    this.engine.seek(time);
  }

  // ── Volume / Rate ──

  setVolume(value: number): void {
    this._masterVolume = value;
    this.engine.setVolume(value);
  }

  getVolume(): number {
    return this.engine.getVolume();
  }

  setRate(value: number): void {
    this.engine.setRate(value);
  }

  getRate(): number {
    return this.engine.getRate();
  }

  setReplayGain(gain: number): void {
    this.engine.setReplayGain?.(gain);
  }

  // ── Sink / EQ / Spectrum ──

  async setSinkId(deviceId: string): Promise<void> {
    await this.engine.setSinkId(deviceId);
  }

  setFilterGain(index: number, value: number): void {
    this.engine.setFilterGain?.(index, value);
  }

  getFilterGains(): number[] {
    return this.engine.getFilterGains?.() ?? [];
  }

  setHighPassFilter(frequency: number, rampTime = 0): void {
    this.engine.setHighPassFilter?.(frequency, rampTime);
  }

  setHighPassQ(q: number): void {
    this.engine.setHighPassQ?.(q);
  }

  setLowPassFilter(frequency: number, rampTime = 0): void {
    this.engine.setLowPassFilter?.(frequency, rampTime);
  }

  setLowPassQ(q: number): void {
    this.engine.setLowPassQ?.(q);
  }

  getFrequencyData(): Uint8Array {
    return this.engine.getFrequencyData?.() ?? new Uint8Array(0);
  }

  getLowFrequencyVolume(): number {
    return this.engine.getLowFrequencyVolume?.() ?? 0;
  }

  // ── State getters ──

  get duration(): number {
    return this.engine.duration;
  }

  get currentTime(): number {
    return this.engine.currentTime;
  }

  get paused(): boolean {
    return this.engine.paused;
  }

  get src(): string {
    return this.engine.src;
  }

  getErrorCode(): number {
    return this.engine.getErrorCode();
  }

  // ── Helpers ──

  togglePlayPause(): void {
    if (this.paused) {
      this.resume();
    } else {
      this.pause();
    }
  }

  private clearPendingSwitch(): void {
    if (this.pendingSwitchTimer) {
      clearTimeout(this.pendingSwitchTimer);
      this.pendingSwitchTimer = null;
    }
    this.engine.setHighPassFilter?.(0, 0);
    this.engine.setHighPassQ?.(0.707);
    if (this.pendingEngine) {
      try {
        this.pendingEngine.destroy();
      } catch {
        // ignore
      }
      this.pendingEngine = null;
    }
  }
}

// ── Singleton ──

const AUDIO_MANAGER_KEY = "__NONSPLAYER_AUDIO_MANAGER__";

export const useAudioManager = (): AudioManager => {
  const win = window as Window & { [AUDIO_MANAGER_KEY]?: AudioManager };
  if (!win[AUDIO_MANAGER_KEY]) {
    win[AUDIO_MANAGER_KEY] = new AudioManager();
    console.log(`[AudioManager] Created, engine: ${win[AUDIO_MANAGER_KEY]!.engineType}`);
  }
  return win[AUDIO_MANAGER_KEY]!;
};
