import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Song } from "$lib/types/song";

type PlayMode = "sequential" | "shuffle" | "single_loop" | "list_loop";

class PlayerService {
  currentSong = $state<Song | null>(null);
  isPlaying = $state(false);
  position = $state(0);
  duration = $state(0);
  volume = $state(0.8);
  playMode = $state<PlayMode>("sequential");
  queue = $state<Song[]>([]);

  private unlisteners: UnlistenFn[] = [];

  constructor() {
    this.setupListeners();
  }

  private async setupListeners() {
    const u1 = await listen<Song>("track-changed", (e) => {
      this.currentSong = e.payload;
      this.duration = e.payload.duration;
    });

    const u2 = await listen<{ state: string }>("player-state-changed", (e) => {
      this.isPlaying = e.payload.state === "playing";
    });

    this.unlisteners = [u1, u2];

    // Poll position while playing (BASS doesn't push position automatically)
    this.startPositionPoll();
  }

  private positionInterval: ReturnType<typeof setInterval> | null = null;

  private startPositionPoll() {
    this.positionInterval = setInterval(async () => {
      if (this.isPlaying) {
        try {
          this.position = await invoke("get_position");
        } catch {
          // engine not available
        }
      }
    }, 250);
  }

  // -- Commands --

  async play(songs: Song[], startIndex = 0) {
    if (songs.length === 0) return;
    const song = songs[startIndex];
    if (!song) return;

    await invoke("play", {
      adapter: song.adapterSlug,
      songId: song.id,
      queueSongs: songs,
      startIndex,
    });
    this.isPlaying = true;
  }

  async pause() {
    await invoke("pause");
    this.isPlaying = false;
  }

  async resume() {
    await invoke("resume");
    this.isPlaying = true;
  }

  async togglePlayback() {
    await invoke("toggle_playback");
  }

  async seek(seconds: number) {
    await invoke("seek", { seconds });
    this.position = seconds;
  }

  async setVolume(vol: number) {
    await invoke("set_volume", { vol: Math.max(0, Math.min(1, vol)) });
    this.volume = vol;
  }

  async next() {
    await invoke("next");
  }

  async prev() {
    await invoke("prev");
  }

  async setPlayMode(mode: PlayMode) {
    await invoke("set_play_mode", { mode });
    this.playMode = mode;
  }

  async getQueue(): Promise<Song[]> {
    return invoke("get_queue");
  }

  destroy() {
    this.unlisteners.forEach((u) => u());
    if (this.positionInterval) clearInterval(this.positionInterval);
  }
}

export const playerService = new PlayerService();
