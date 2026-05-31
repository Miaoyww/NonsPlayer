import type { AdapterMetadata, CapabilityType, AdapterConfig } from "$lib/types/adapter";
import { listAdapters, initAdapters, scanLocal } from "$lib/services/adapter-service";

// ── Frontend-only adapters (no Rust backend) ──────────────────────────

const FRONTEND_ADAPTER_METADATA: AdapterMetadata[] = [
  {
    slug: "netease",
    platform: "netease",
    displayPlatform: "网易云音乐",
    author: "NonsPlayer",
    description: "网易云音乐适配器，支持搜索、歌单、每日推荐、二维码登录",
    version: "0.3.0",
    capabilities: ["Music", "Search", "Album", "Artist", "Playlist", "Account", "Recommend"],
  },
];

class AdapterStore {
  adapters = $state<AdapterMetadata[]>([]);

  /** Initialize all adapters from config. Called once at app startup. */
  async initialize(config: AdapterConfig) {
    const list = await initAdapters(config);
    // Merge frontend-only adapters with backend adapters
    this.adapters = [...FRONTEND_ADAPTER_METADATA, ...list];
  }

  /** Re-scan local music folders and re-register. */
  async rescanLocal(dirs: string[]) {
    const list = await scanLocal(dirs);
    this.adapters = [...FRONTEND_ADAPTER_METADATA, ...list];
  }

  /** Refresh adapter list from backend. */
  async refresh() {
    const list = await listAdapters();
    // Merge frontend-only adapters with backend adapters
    this.adapters = [...FRONTEND_ADAPTER_METADATA, ...list];
  }

  /** Get adapters that support a given capability. */
  byCapability(cap: CapabilityType): AdapterMetadata[] {
    return this.adapters.filter((a) => a.capabilities.includes(cap));
  }

  get streaming(): AdapterMetadata[] {
    return this.byCapability("Account");
  }

  /** Look up a single adapter by slug. */
  get(slug: string): AdapterMetadata | undefined {
    return this.adapters.find((a) => a.slug === slug);
  }
}

export const adapterStore = new AdapterStore();
