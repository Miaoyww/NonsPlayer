import { usePlayerController, type PlayMode, type PlayerState } from "$lib/player/PlayerController";
import type { Song } from "$lib/types/song";

/**
 * PlayerService — thin Svelte 5 reactive wrapper around PlayerController.
 *
 * Uses $state runes so that Svelte components can reactively bind to
 * playback state. All actual playback logic lives in PlayerController.
 */
class PlayerService {
  private controller = usePlayerController();

  // ── Reactive state ──

  currentSong = $state<Song | null>(null);
  isPlaying = $state(false);
  position = $state(0);
  duration = $state(0);
  volume = $state(0.8);
  playMode = $state<PlayMode>("sequential");
  queue = $state<Song[]>([]);
  queueIndex = $state(0);
  loading = $state(false);

  // For backward compatibility
  buffering = $state(false);
  downloadPercent = $state(0);
  isLive = $state(false);

  private unsub: (() => void) | null = null;

  constructor() {
    this.unsub = this.controller.subscribe((state: PlayerState) => {
      this.currentSong = state.currentSong;
      this.isPlaying = state.isPlaying;
      this.position = state.position;
      this.duration = state.duration;
      this.volume = state.volume;
      this.playMode = state.playMode;
      this.queue = state.queue;
      this.queueIndex = state.queueIndex;
      this.loading = state.loading;
    });
  }

  // ── Commands ──

  async play(songs: Song[], startIndex = 0): Promise<void> {
    await this.controller.play(songs, startIndex);
  }

  async pause(): Promise<void> {
    this.controller.pause();
  }

  async resume(): Promise<void> {
    await this.controller.resume();
  }

  async togglePlayback(): Promise<void> {
    await this.controller.togglePlayback();
  }

  async seek(seconds: number): Promise<void> {
    this.controller.seek(seconds);
  }

  async setVolume(vol: number): Promise<void> {
    this.controller.setVolume(vol);
  }

  async next(): Promise<void> {
    await this.controller.next();
  }

  async prev(): Promise<void> {
    await this.controller.prev();
  }

  async setPlayMode(mode: PlayMode): Promise<void> {
    this.controller.setPlayMode(mode);
  }

  async jumpTo(index: number): Promise<void> {
    await this.controller.jumpTo(index);
  }

  // ── Cleanup ──

  destroy(): void {
    this.unsub?.();
    this.controller.destroy();
  }
}

export const playerService = new PlayerService();
