import type { AdapterMetadata, CapabilityType } from "$lib/types/adapter";
import { listAdapters, initAdapters, scanLocal } from "$lib/services/adapter-service";

class AdapterStore {
  adapters = $state<AdapterMetadata[]>([]);

  /** Initialize all adapters from config. Called once at app startup. */
  async initialize(dirs: string[]) {
    const list = await initAdapters({ localMusicDirs: dirs });
    this.adapters = list;
  }

  /** Re-scan local music folders and re-register. */
  async rescanLocal(dirs: string[]) {
    const list = await scanLocal(dirs);
    this.adapters = list;
  }

  /** Refresh adapter list from backend. */
  async refresh() {
    this.adapters = await listAdapters();
  }

  /** Get adapters that support a given capability. */
  byCapability(cap: CapabilityType): AdapterMetadata[] {
    // The frontend doesn't know per-adapter capabilities yet;
    // this will be filled in when the Rust adapter returns capabilities in metadata.
    return this.adapters.filter((a) => {
      // For now, all registered adapters support Music. Refine later.
      return true;
    });
  }

  /** Look up a single adapter by slug. */
  get(slug: string): AdapterMetadata | undefined {
    return this.adapters.find((a) => a.slug === slug);
  }
}

export const adapterStore = new AdapterStore();
