export { usePlayerController, type PlayMode, type QueueItem, type PlayerState } from "./PlayerController";
export { useAudioManager } from "./AudioManager";
export { useSongManager, type AudioSource } from "./SongManager";
export { AudioErrorCode, AUDIO_EVENTS } from "./engine/BaseAudioPlayer";
export type { IPlaybackEngine, EngineCapabilities, PlayOptions, PauseOptions, FadeCurve } from "./engine/types";
