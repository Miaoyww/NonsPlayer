/**
 * Engine capability descriptor.
 */
export interface EngineCapabilities {
  supportsRate: boolean;
  supportsSinkId: boolean;
  supportsEqualizer: boolean;
  supportsSpectrum: boolean;
}

export type FadeCurve = "linear" | "exponential" | "equalPower";

export interface PlayOptions {
  autoPlay?: boolean;
  fadeIn?: boolean;
  fadeDuration?: number;
  fadeCurve?: FadeCurve;
  seek?: number;
}

export interface PauseOptions {
  fadeOut?: boolean;
  fadeDuration?: number;
  fadeCurve?: FadeCurve;
  keepContextRunning?: boolean;
}

export const AUDIO_EVENTS = {
  PLAY: "play",
  PAUSE: "pause",
  ENDED: "ended",
  TIME_UPDATE: "timeupdate",
  ERROR: "error",
  CAN_PLAY: "canplay",
  LOAD_START: "loadstart",
  SEEKED: "seeked",
  WAITING: "waiting",
  VOLUME_CHANGE: "volumechange",
  PLAYING: "playing",
  SEEKING: "seeking",
  EMPTIED: "emptied",
} as const;

export type AudioEventType = (typeof AUDIO_EVENTS)[keyof typeof AUDIO_EVENTS];

export interface AudioErrorDetail {
  originalEvent?: Event;
  errorCode: number;
  message?: string;
}

export type AudioEventMap = {
  [K in AudioEventType]: K extends typeof AUDIO_EVENTS.ERROR
    ? CustomEvent<AudioErrorDetail>
    : CustomEvent<undefined>;
};

export enum AudioErrorCode {
  ABORTED = 1,
  NETWORK = 2,
  DECODE = 3,
  SRC_NOT_SUPPORTED = 4,
  DOM_ABORT = 20,
}

/**
 * Unified playback engine interface.
 * All audio engines (HTML5, FFmpeg WASM, MPV, etc.) implement this.
 */
export interface IPlaybackEngine {
  init(): void;
  destroy(): void;

  play(url?: string, options?: PlayOptions): Promise<void>;
  resume(options?: { fadeIn?: boolean; fadeDuration?: number }): Promise<void>;
  pause(options?: PauseOptions): void;
  stop(): void;
  seek(time: number): void;

  readonly duration: number;
  readonly currentTime: number;
  readonly paused: boolean;
  readonly src: string;

  setVolume(value: number): void;
  getVolume(): number;
  rampVolumeTo?(value: number, duration: number, curve?: FadeCurve): void;

  setRate(rate: number): void;
  getRate(): number;

  setSinkId(deviceId: string): Promise<void>;

  setFilterGain?(index: number, value: number): void;
  getFilterGains?(): number[];

  setHighPassFilter?(frequency: number, rampTime?: number): void;
  setHighPassQ?(q: number): void;
  setLowPassFilter?(frequency: number, rampTime?: number): void;
  setLowPassQ?(q: number): void;

  getFrequencyData?(): Uint8Array;
  getLowFrequencyVolume?(): number;

  setReplayGain?(gain: number): void;
  setPitchShift?(semitones: number): void;

  getErrorCode(): number;

  addEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | AddEventListenerOptions,
  ): void;
  removeEventListener(
    type: string,
    listener: EventListenerOrEventListenerObject,
    options?: boolean | EventListenerOptions,
  ): void;

  readonly capabilities: EngineCapabilities;
}
