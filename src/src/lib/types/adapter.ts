import type { Song } from "./song";
import type { Album } from "./album";
import type { Artist } from "./artist";
import type { Playlist } from "./playlist";
import type { Account } from "./account";

export interface AdapterMetadata {
  slug: string;
  platform: string;
  displayPlatform: string;
  author: string;
  description: string;
  version: string;
  capabilities: CapabilityType[];
}

export type CapabilityType =
  | "Music"
  | "Search"
  | "Album"
  | "Artist"
  | "Playlist"
  | "Account"
  | "Recommend";

export interface SearchResult {
  songs: Song[];
  albums: Album[];
  artists: Artist[];
  playlists: Playlist[];
}

export type LoginStatus =
  | { status: "waiting"; qr_url: string }
  | { status: "scanned" }
  | { status: "confirmed"; account: Account }
  | { status: "timeout" }
  | { status: "cancelled" };

export interface AdapterConfig {
  localMusicDirs?: string[];
}
