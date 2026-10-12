// One cursor and in-flight page per source. Batches append in delivery order;
// changing a filter never reconstructs or sorts already delivered results.
export interface SearchBatch<T> {
  items: T[]
  nextCursor: string | null
}
export interface SearchSource {
  source: string
  name: string
  unavailable?: string
}
export interface SourceProgress extends SearchSource {
  cursor: string | null
  count: number
  busy: boolean
  more: boolean
  error?: string
  capped: boolean
}
export interface AggregateSnapshot<T> {
  sources: SourceProgress[]
  items: { source: string; item: T }[]
}

export class AggregateQuery<T> {
  private active = true
  private generation = 0
  private listeners = new Set<() => void>()
  private seen = new Set<string>()
  private state: AggregateSnapshot<T>
  constructor(
    sources: SearchSource[],
    private loadPage: (source: string, cursor: string | null) => Promise<SearchBatch<T>>,
    private identity: (item: T) => string,
    private describeError: (cause: unknown) => string,
    private limit = 1000
  ) {
    this.state = {
      items: [],
      sources: sources.map((value) => ({
        ...value,
        cursor: null,
        count: 0,
        busy: false,
        more: !value.unavailable,
        capped: false
      }))
    }
  }
  getSnapshot = () => this.state
  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }
  start() {
    this.active = true
    this.publish()
  }
  dispose() {
    this.active = false
    this.generation++
    this.state = {
      ...this.state,
      sources: this.state.sources.map((value) => ({ ...value, busy: false }))
    }
  }
  private publish() {
    this.state = {
      items: this.state.items,
      sources: this.state.sources.map((value) => ({ ...value }))
    }
    this.listeners.forEach((listener) => listener())
  }
  async load(source: string, retry = false) {
    const progress = this.state.sources.find((value) => value.source === source)
    if (!this.active || !progress || progress.busy || !progress.more || (progress.error && !retry))
      return
    progress.busy = true
    progress.error = undefined
    const generation = this.generation
    this.publish()
    try {
      const batch = await this.loadPage(source, progress.cursor)
      if (!this.active || generation !== this.generation) return
      if (batch.nextCursor !== null && batch.nextCursor === progress.cursor)
        throw new Error('来源返回了重复游标，请重试。')
      const appended: { source: string; item: T }[] = []
      for (const item of batch.items) {
        const key = JSON.stringify([source, this.identity(item)])
        if (this.seen.has(key) || progress.count >= this.limit) continue
        this.seen.add(key)
        appended.push({ source, item })
        progress.count++
      }
      this.state.items = [...this.state.items, ...appended]
      progress.cursor = batch.nextCursor
      progress.capped = progress.count >= this.limit && batch.nextCursor !== null
      progress.more = batch.nextCursor !== null && !progress.capped
    } catch (cause) {
      if (this.active && generation === this.generation) progress.error = this.describeError(cause)
    } finally {
      if (this.active && generation === this.generation) {
        // publish clones the progress records; use the live copy for settlement.
        const live = this.state.sources.find((value) => value.source === source)!
        Object.assign(live, progress, { busy: false })
        this.publish()
      }
    }
  }
  loadVisible(filter: string) {
    for (const progress of this.state.sources)
      if (filter === 'all' || progress.source === filter) void this.load(progress.source)
  }
}
