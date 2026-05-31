import {
  BaseAudioPlayer,
  AUDIO_EVENTS,
  AudioErrorCode,
} from "./BaseAudioPlayer";
import type { EngineCapabilities } from "./types";
import type { AudioEventType } from "./types";

/**
 * HTML5 Audio element based playback engine.
 *
 * Uses a native `<audio>` element for decoding, bridged into the
 * Web Audio API graph via `createMediaElementSource()`.
 */
export class AudioElementPlayer extends BaseAudioPlayer {
  private audioElement: HTMLAudioElement;
  private sourceNode: MediaElementAudioSourceNode | null = null;

  private isInternalSeeking = false;
  private targetSeekTime = 0;

  readonly capabilities: EngineCapabilities = {
    supportsRate: true,
    supportsSinkId: true,
    supportsEqualizer: true,
    supportsSpectrum: true,
  };

  constructor() {
    super();
    this.audioElement = new Audio();
    this.audioElement.crossOrigin = "anonymous";
    this.bindInternalEvents();

    this.audioElement.addEventListener("seeked", () => {
      this.isInternalSeeking = false;
    });
  }

  // ── Graph connection ──

  protected onGraphInitialized(): void {
    if (!this.audioCtx || !this.inputNode) return;

    try {
      if (!this.sourceNode) {
        this.sourceNode = this.audioCtx.createMediaElementSource(this.audioElement);
      } else {
        this.sourceNode.disconnect();
      }
      this.sourceNode.connect(this.inputNode);
    } catch (error) {
      console.error("[AudioElementPlayer] Failed to create MediaElementSource:", error);
    }
  }

  // ── Load / Play / Pause ──

  async load(url: string): Promise<void> {
    this.audioElement.src = url;
    this.audioElement.load();
  }

  protected async doPlay(): Promise<void> {
    return this.audioElement.play();
  }

  protected doPause(): void {
    this.audioElement.pause();
  }

  // ── Seek ──

  async seek(time: number): Promise<void> {
    this.isInternalSeeking = true;
    this.targetSeekTime = time;
    this.cancelPendingPause();
    this.doSeek(time);
  }

  protected doSeek(time: number): void {
    if (Number.isFinite(time)) {
      this.audioElement.currentTime = time;
    }
  }

  // ── Stop ──

  stop(): void {
    super.stop();
    this.audioElement.removeAttribute("src");
    this.audioElement.load();
  }

  // ── Rate ──

  setRate(value: number): void {
    this.audioElement.playbackRate = value;
    this.audioElement.defaultPlaybackRate = value;
  }

  getRate(): number {
    return this.audioElement.playbackRate;
  }

  // ── Pitch ──

  setPitchShift(semitones: number): void {
    if ("preservesPitch" in this.audioElement) {
      const el = this.audioElement as HTMLAudioElement & { preservesPitch: boolean };
      el.preservesPitch = semitones === 0;
    }
  }

  // ── Sink ──

  protected async doSetSinkId(deviceId: string): Promise<void> {
    if (typeof (this.audioElement as any).setSinkId === "function") {
      await (this.audioElement as any).setSinkId(deviceId);
    }
  }

  // ── State getters ──

  get src(): string {
    return this.audioElement.src || "";
  }

  get duration(): number {
    return this.audioElement.duration || 0;
  }

  get currentTime(): number {
    if (this.isInternalSeeking) {
      return this.targetSeekTime;
    }
    return (
      (this.audioElement.currentTime || 0) -
      this.compensatedLatency +
      this.audioDelayCompensation / 1000
    );
  }

  get paused(): boolean {
    return this.audioElement.paused;
  }

  getErrorCode(): number {
    if (!this.audioElement.error) return 0;
    switch (this.audioElement.error.code) {
      case MediaError.MEDIA_ERR_ABORTED:
        return AudioErrorCode.ABORTED;
      case MediaError.MEDIA_ERR_NETWORK:
        return AudioErrorCode.NETWORK;
      case MediaError.MEDIA_ERR_DECODE:
        return AudioErrorCode.DECODE;
      case MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED:
        return AudioErrorCode.SRC_NOT_SUPPORTED;
      default:
        return 0;
    }
  }

  // ── Event binding ──

  private bindInternalEvents(): void {
    const events: AudioEventType[] = Object.values(AUDIO_EVENTS);

    for (const eventType of events) {
      this.audioElement.addEventListener(eventType, (e) => {
        if (eventType === AUDIO_EVENTS.ERROR) {
          this.dispatch(AUDIO_EVENTS.ERROR, {
            originalEvent: e,
            errorCode: this.getErrorCode(),
          });
        } else {
          this.dispatch(eventType);
        }
      });
    }
  }
}
