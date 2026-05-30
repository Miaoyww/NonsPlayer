import type { Song } from "$lib/types/song";
import { useAudioManager } from "./AudioManager";
import { useSongManager, type AudioSource } from "./SongManager";
import { AudioErrorCode } from "./engine/BaseAudioPlayer";

// ── Types ──

export type PlayMode = "sequential" | "shuffle" | "single_loop" | "list_loop";

export interface QueueItem {
  song: Song;
  adapterSlug: string;
}

export interface PlayerState {
  currentSong: Song | null;
  isPlaying: boolean;
  position: number;
  duration: number;
  volume: number;
  playMode: PlayMode;
  queue: Song[];
  queueIndex: number;
  loading: boolean;
}

type PlayerStateListener = (state: PlayerState) => void;

// ── DJ filter keywords ──

const DJ_MODE_KEYWORDS = [
  "DJ", "DJ版", "DJ伴奏", "抖音", "抖音版", "抖音热歌",
  "DJ开场", "抖音神曲", "DJ舞曲", "电音", "车载DJ",
];

// ── Shuffle helper ──

function shuffleArray<T>(arr: T[]): T[] {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

/**
 * PlayerController — the central playback orchestrator.
 *
 * Manages the complete playback lifecycle inspired by SPlayer's architecture:
 * song resolution → engine playback → event handling → auto-advance.
 *
 * Singleton, accessible via `usePlayerController()`.
 */
class PlayerController {
  // ── Dependencies ──
  private audioManager = useAudioManager();
  private songManager = useSongManager();

  // ── Queue state ──
  private _queue: QueueItem[] = [];
  private _queueIndex = 0;
  private _playMode: PlayMode = "sequential";
  private shuffleOrder: number[] = [];
  private shufflePos = 0;

  // ── Player state ──
  private _currentSong: Song | null = null;
  private _isPlaying = false;
  private _position = 0;
  private _duration = 0;
  private _volume = 0.8;
  private _loading = false;

  // ── Error handling ──
  private currentRequestToken = 0;
  private retryInfo: { songId: string; count: number } = { songId: "", count: 0 };
  private readonly MAX_RETRY = 3;
  private failSkipCount = 0;
  private lastErrorTime = 0;

  // ── Listeners ──
  private stateListeners: Set<PlayerStateListener> = new Set();
  private boundTimeUpdate: (() => void) | null = null;
  private boundEnded: (() => void) | null = null;
  private boundError: ((e: Event) => void) | null = null;

  // ── Initialization ──

  constructor() {
    this.audioManager.init();
    this.bindAudioEvents();
  }

  private bindAudioEvents(): void {
    // Timeupdate — throttled to ~200ms equivalent via rAF gate
    let lastUpdate = 0;
    this.boundTimeUpdate = () => {
      const now = performance.now();
      if (now - lastUpdate < 200) return;
      lastUpdate = now;

      this._position = this.audioManager.currentTime;
      this._duration = this.audioManager.duration || this._duration;
      this.notifyState();
    };
    this.audioManager.addEventListener("timeupdate", this.boundTimeUpdate);

    // Track ended → auto-advance
    this.boundEnded = () => {
      console.log(`[PlayerController] Track ended: "${this._currentSong?.name}"`);
      this.nextOrPrev("next", true, true);
    };
    this.audioManager.addEventListener("ended", this.boundEnded);

    // Error handling
    this.boundError = (e: Event) => {
      const detail = (e as CustomEvent).detail;
      this.handlePlaybackError(detail?.errorCode);
    };
    this.audioManager.addEventListener("error", this.boundError);

    // Play / Pause state sync
    this.audioManager.addEventListener("play", () => {
      this._isPlaying = true;
      this._loading = false;
      this.notifyState();
    });
    this.audioManager.addEventListener("pause", () => {
      this._isPlaying = false;
      this.notifyState();
    });
    this.audioManager.addEventListener("loadstart", () => {
      this._loading = true;
      this.notifyState();
    });
    this.audioManager.addEventListener("canplay", () => {
      this._loading = false;
      this.notifyState();
    });
  }

  // ── State snapshot ──

  private snapshot(): PlayerState {
    return {
      currentSong: this._currentSong,
      isPlaying: this._isPlaying,
      position: this._position,
      duration: this._duration,
      volume: this._volume,
      playMode: this._playMode,
      queue: this._queue.map((q) => q.song),
      queueIndex: this._queueIndex,
      loading: this._loading,
    };
  }

  private notifyState(): void {
    const state = this.snapshot();
    for (const listener of this.stateListeners) {
      listener(state);
    }
  }

  /** Subscribe to state changes. Returns unsubscribe function. */
  subscribe(listener: PlayerStateListener): () => void {
    this.stateListeners.add(listener);
    // Push initial state
    listener(this.snapshot());
    return () => {
      this.stateListeners.delete(listener);
    };
  }

  // ── Getters ──

  get currentSong(): Song | null { return this._currentSong; }
  get isPlaying(): boolean { return this._isPlaying; }
  get position(): number { return this._position; }
  get duration(): number { return this._duration; }
  get volume(): number { return this._volume; }
  get playMode(): PlayMode { return this._playMode; }
  get queue(): Song[] { return this._queue.map((q) => q.song); }
  get queueIndex(): number { return this._queueIndex; }
  get loading(): boolean { return this._loading; }

  // ── Core: Play ──

  /**
   * Load songs into the queue and start playing from startIndex.
   */
  async play(songs: Song[], startIndex = 0): Promise<void> {
    if (songs.length === 0) return;

    // Build queue
    this._queue = songs.map((s) => ({
      song: s,
      adapterSlug: s.adapterSlug || "",
    }));
    this._queueIndex = Math.max(0, Math.min(startIndex, songs.length - 1));
    this.shuffleOrder = [];
    this.shufflePos = 0;

    if (this._playMode === "shuffle") {
      this.generateShuffle();
    }

    await this.playCurrent();
  }

  /** Play the song at the current queue position. */
  private async playCurrent(): Promise<void> {
    const item = this._queue[this._queueIndex];
    if (!item) return;

    this.currentRequestToken++;
    const token = this.currentRequestToken;

    const song = item.song;
    console.log(`[PlayerController] Playing: "${song.name}"`);

    // DJ filter
    if (this.shouldSkipSong(song)) {
      console.log(`[PlayerController] Skipping DJ song: "${song.name}"`);
      this.nextOrPrev("next");
      return;
    }

    try {
      this._loading = true;
      this._currentSong = song;
      this._position = 0;
      this._duration = song.duration || 0;
      this.notifyState();

      // Reset retry for new song
      const songId = song.id;
      if (this.retryInfo.songId !== songId) {
        this.retryInfo = { songId, count: 0 };
      }

      // Stop current playback (non-crossfade)
      this.audioManager.stop();

      // Resolve URL
      const audioSource = await this.songManager.getAudioSource(song);

      if (token !== this.currentRequestToken) return;

      if (!audioSource.url) {
        throw new Error("AUDIO_SOURCE_EMPTY");
      }

      // Load and play
      await this.audioManager.play(audioSource.url, {
        autoPlay: true,
        seek: 0,
      });

      if (token !== this.currentRequestToken) return;

      this._isPlaying = true;
      this._loading = false;
      this.notifyState();

      // Prefetch next
      const nextItem = this.peekNext();
      if (nextItem) {
        this.songManager.prefetchNextSong(nextItem.song);
      }
    } catch (error) {
      if (token === this.currentRequestToken) {
        console.error("[PlayerController] Play failed:", error);
        this.handlePlaybackError(undefined);
      }
    }
  }

  // ── Queue navigation ──

  private peekNext(): QueueItem | null {
    if (this._queue.length === 0) return null;

    switch (this._playMode) {
      case "single_loop":
        return this._queue[this._queueIndex] ?? null;
      case "shuffle": {
        const nextPos = this.shufflePos + 1;
        if (nextPos < this.shuffleOrder.length) {
          return this._queue[this.shuffleOrder[nextPos]] ?? null;
        }
        return null;
      }
      case "list_loop": {
        const nextIdx = (this._queueIndex + 1) % this._queue.length;
        return this._queue[nextIdx] ?? null;
      }
      default: { // sequential
        const nextIdx = this._queueIndex + 1;
        return nextIdx < this._queue.length ? this._queue[nextIdx] : null;
      }
    }
  }

  /**
   * Move to next or previous track and play it.
   */
  async nextOrPrev(
    direction: "next" | "prev" = "next",
    play = true,
    autoEnd = false,
  ): Promise<void> {
    if (this._queue.length === 0) return;

    // Single-loop with auto-end → replay current
    if (this._playMode === "single_loop" && autoEnd) {
      await this.playCurrent();
      return;
    }

    // Calculate next index
    let nextIndex: number;
    if (this._playMode === "shuffle") {
      if (direction === "next") {
        this.shufflePos++;
        if (this.shufflePos >= this.shuffleOrder.length) {
          this._loading = false;
          this.notifyState();
          return; // end of shuffle
        }
        nextIndex = this.shuffleOrder[this.shufflePos];
      } else {
        if (this.shufflePos <= 0) return; // can't go back
        this.shufflePos--;
        nextIndex = this.shuffleOrder[this.shufflePos];
      }
    } else {
      if (direction === "next") {
        nextIndex = this._queueIndex + 1;
        if (nextIndex >= this._queue.length) {
          if (this._playMode === "list_loop") {
            nextIndex = 0;
          } else {
            this._loading = false;
            this.notifyState();
            return; // end of queue
          }
        }
      } else {
        nextIndex = this._queueIndex - 1;
        if (nextIndex < 0) {
          if (this._playMode === "list_loop") {
            nextIndex = this._queue.length - 1;
          } else {
            nextIndex = 0; // stay at first
          }
        }
      }
    }

    // DJ filter: skip unwanted songs
    let attempts = 0;
    while (attempts < this._queue.length) {
      const nextSong = this._queue[nextIndex]?.song;
      if (!nextSong || !this.shouldSkipSong(nextSong)) break;
      nextIndex = direction === "next"
        ? (nextIndex + 1) % this._queue.length
        : (nextIndex - 1 + this._queue.length) % this._queue.length;
      attempts++;
    }

    this._queueIndex = nextIndex;

    if (play) {
      await this.playCurrent();
    } else {
      this.notifyState();
    }
  }

  async next(): Promise<void> {
    await this.nextOrPrev("next");
  }

  async prev(): Promise<void> {
    await this.nextOrPrev("prev");
  }

  /** Jump to a specific queue index. */
  async jumpTo(index: number): Promise<void> {
    if (index < 0 || index >= this._queue.length) return;
    this._queueIndex = index;
    if (this._playMode === "shuffle") {
      this.shufflePos = this.shuffleOrder.indexOf(index);
      if (this.shufflePos < 0) this.shufflePos = 0;
    }
    await this.playCurrent();
  }

  // ── Play controls ──

  async togglePlayback(): Promise<void> {
    if (this._isPlaying) {
      this.audioManager.pause();
      this._isPlaying = false;
    } else if (this.audioManager.src) {
      await this.audioManager.resume();
      this._isPlaying = true;
    } else if (this._currentSong) {
      await this.playCurrent();
    }
    this.notifyState();
  }

  pause(): void {
    this.audioManager.pause();
    this._isPlaying = false;
    this.notifyState();
  }

  async resume(): Promise<void> {
    await this.audioManager.resume();
    this._isPlaying = true;
    this.notifyState();
  }

  seek(seconds: number): void {
    this._position = seconds;
    this.audioManager.seek(seconds);
    this.notifyState();
  }

  setVolume(vol: number): void {
    this._volume = Math.max(0, Math.min(1, vol));
    this.audioManager.setVolume(this._volume);
    this.notifyState();
  }

  // ── Play mode ──

  setPlayMode(mode: PlayMode): void {
    this._playMode = mode;
    if (mode === "shuffle") {
      this.generateShuffle();
    } else {
      this.shuffleOrder = [];
      this.shufflePos = 0;
    }
    this.notifyState();
  }

  cyclePlayMode(): void {
    const modes: PlayMode[] = ["sequential", "shuffle", "single_loop", "list_loop"];
    const idx = modes.indexOf(this._playMode);
    this.setPlayMode(modes[(idx + 1) % modes.length]);
  }

  private generateShuffle(): void {
    const indices = Array.from({ length: this._queue.length }, (_, i) => i);
    this.shuffleOrder = shuffleArray(indices);
    this.shufflePos = 0;
    this._queueIndex = this.shuffleOrder[0] ?? 0;
  }

  // ── Error handling ──

  private handlePlaybackError(errCode: number | undefined): void {
    const now = Date.now();
    if (now - this.lastErrorTime < 200) return; // debounce
    this.lastErrorTime = now;

    this.songManager.clearPrefetch();

    const songId = this._currentSong?.id || "";

    // Reset retry for new song
    if (this.retryInfo.songId !== songId) {
      this.retryInfo = { songId, count: 0 };
    }

    if (this.retryInfo.count >= 3) {
      console.error(`[PlayerController] Max retries exceeded for "${this._currentSong?.name}"`);
      this._loading = false;
      this.retryInfo.count = 0;
      this.skipToNextWithDelay();
      return;
    }

    // User-initiated abort
    if (errCode === AudioErrorCode.ABORTED || errCode === AudioErrorCode.DOM_ABORT) {
      this.retryInfo.count = 0;
      return;
    }

    // Unsupported format
    if (errCode === AudioErrorCode.SRC_NOT_SUPPORTED) {
      console.warn(`[PlayerController] Unsupported format, skipping`);
      this._loading = false;
      this.retryInfo.count = 0;
      this.skipToNextWithDelay();
      return;
    }

    // Retry
    this.retryInfo.count++;
    console.warn(
      `[PlayerController] Retry ${this.retryInfo.count}/${this.MAX_RETRY} for "${this._currentSong?.name}"`,
    );

    if (this.retryInfo.count <= this.MAX_RETRY) {
      setTimeout(() => {
        this.playCurrent();
      }, 1000);
    } else {
      this.retryInfo.count = 0;
      this.skipToNextWithDelay();
    }
  }

  private async skipToNextWithDelay(): Promise<void> {
    this.failSkipCount++;

    if (this.failSkipCount >= 3) {
      console.error("[PlayerController] Too many failures, stopping");
      this._loading = false;
      this._isPlaying = false;
      this.failSkipCount = 0;
      this.notifyState();
      return;
    }

    if (this._queue.length <= 1) {
      console.error("[PlayerController] Only one song in queue, stopping");
      this._loading = false;
      this.failSkipCount = 0;
      this.notifyState();
      return;
    }

    await new Promise((r) => setTimeout(r, 500));
    await this.nextOrPrev("next");
  }

  // ── DJ filter ──

  shouldSkipSong(song: Song): boolean {
    const name = (song.name || "").toUpperCase();
    return DJ_MODE_KEYWORDS.some((kw) => name.includes(kw.toUpperCase()));
  }

  // ── Cleanup ──

  destroy(): void {
    if (this.boundTimeUpdate) {
      this.audioManager.removeEventListener("timeupdate", this.boundTimeUpdate);
    }
    if (this.boundEnded) {
      this.audioManager.removeEventListener("ended", this.boundEnded);
    }
    if (this.boundError) {
      this.audioManager.removeEventListener("error", this.boundError);
    }
    this.audioManager.destroy();
    this.stateListeners.clear();
  }
}

// ── Singleton ──

const PLAYER_CONTROLLER_KEY = "__NONSPLAYER_PLAYER_CONTROLLER__";

export const usePlayerController = (): PlayerController => {
  const win = window as Window & { [PLAYER_CONTROLLER_KEY]?: PlayerController };
  if (!win[PLAYER_CONTROLLER_KEY]) {
    win[PLAYER_CONTROLLER_KEY] = new PlayerController();
    console.log("[PlayerController] Created");
  }
  return win[PLAYER_CONTROLLER_KEY]!;
};
