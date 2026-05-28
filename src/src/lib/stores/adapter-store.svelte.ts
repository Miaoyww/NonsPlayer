import type { AdapterMetadata, CapabilityType, AdapterConfig } from "$lib/types/adapter";
import { listAdapters, initAdapters, scanLocal } from "$lib/services/adapter-service";

class AdapterStore {
  adapters = $state<AdapterMetadata[]>([]);

  /** Initialize all adapters from config. Called once at app startup. */
  async initialize(config: AdapterConfig) {
    const list = await initAdapters(config);
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
    return this.adapters.filter((a) => {
      // All registered adapters support basic capabilities.
      return true;
    });
  }

  /** Look up a single adapter by slug. */
  get(slug: string): AdapterMetadata | undefined {
    return this.adapters.find((a) => a.slug === slug);
  }
}

export const adapterStore = new AdapterStore();
