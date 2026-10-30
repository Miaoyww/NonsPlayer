export type StoreKind = 'plugin' | 'adapter'
export type StoreSource = 'cdn' | 'github'

export interface StoreEntry {
  [field: string]: unknown
  id: string
  kind?: StoreKind
  name: string
  version: string
  description?: string | null
  authors?: readonly { name: string; link?: string }[]
  license?: string
  repository?: string
  labels?: readonly string[]
  distribution?: string
  installable?: boolean
  artifact?: { url: string; size?: number; sha256?: string; md5?: string } | null
  release?: { tag?: string; created_at?: string; description?: string; notes?: string }
  releaseNotes?: string
  total_download_count?: number | null
  permissions?: readonly string[] | Record<string, unknown>
  httpHosts?: readonly string[]
  fileRoots?: readonly string[]
  compatible: boolean
  unavailableReason?: string
  installedVersion?: string
  updateAvailable: boolean
  bundled: boolean
  blocked?: boolean
  deprecated?: boolean
  unavailable?: boolean
}

export interface StoreIndex {
  source: StoreSource
  url: string
  generatedAt: string
  plugins: StoreEntry[]
  adapters: StoreEntry[]
}

export interface StorePreview {
  token: string
  kind: StoreKind
  id: string
  name: string
  version: string
  installedVersion?: string
  permissions: string[]
  warnings: string[]
}

export interface StoreState {
  source?: StoreSource
  index?: StoreIndex
  loading: boolean
  busy: boolean
  phase?: 'prepare' | 'review' | 'install'
  preparingId?: string
  detail?: { source: StoreSource; entry: StoreEntry }
  review?: { source: StoreSource; preview: StorePreview }
  trusted: boolean
  error?: string
  notice?: string
}

type NativeCall = <T>(command: string, args?: Record<string, unknown>) => Promise<T>

export function unavailableReason(entry: StoreEntry): string | undefined {
  if (entry.distribution === 'source')
    return entry.bundled ? '官方已随包安装' : '作者尚未完成，暂不可安装'
  if (entry.bundled) return '官方已随包安装'
  if (entry.blocked) return entry.unavailableReason || '此扩展已被禁止安装'
  if (!entry.compatible) return entry.unavailableReason || '与当前应用不兼容'
  if (entry.unavailable || entry.installable === false)
    return entry.unavailableReason || '暂不可安装'
  if (entry.installedVersion && !entry.updateAvailable) return '已安装最新版本'
  return entry.unavailableReason || undefined
}

export function searchEntries(entries: readonly StoreEntry[], query: string) {
  const words = query.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean)
  return entries.filter((entry) => {
    const text = [entry.name, entry.id, entry.description, ...(entry.labels ?? [])]
      .join(' ')
      .toLocaleLowerCase()
    return words.every((word) => text.includes(word))
  })
}

export function permissionLines(entry: StoreEntry): string[] {
  const permissions = entry.permissions
  const lines = Array.isArray(permissions)
    ? [...permissions]
    : Object.entries(permissions ?? {}).flatMap(([key, value]) => {
        if (value === false || value == null) return []
        if (Array.isArray(value)) return value.length ? [`${key}：${value.join('、')}`] : []
        return [`${key}：${typeof value === 'object' ? JSON.stringify(value) : String(value)}`]
      })
  if (entry.httpHosts?.length) lines.push(`网络域名：${entry.httpHosts.join('、')}`)
  if (entry.fileRoots?.length) lines.push(`文件范围：${entry.fileRoots.join('、')}`)
  return lines
}

// A request keeps its source and generation even when React changes views or unmounts.
export class ExtensionStoreController {
  private state: StoreState = { loading: false, busy: false, trusted: false }
  private listeners = new Set<() => void>()
  private active = false
  private generation = 0
  private operation = 0
  private pending?: { source?: StoreSource; promise: Promise<void> }

  constructor(
    readonly kind: StoreKind,
    private call: NativeCall,
    private callbacks: {
      onInstalled: () => void | Promise<void>
      onBusyChange: (busy: boolean) => void
      report: (cause: unknown) => void
      errorText: (cause: unknown) => string
    }
  ) {}

  getSnapshot = () => this.state
  subscribe = (listener: () => void) => {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private patch(patch: Partial<StoreState>) {
    const previousBusy = this.state.busy
    this.state = { ...this.state, ...patch }
    if (previousBusy !== this.state.busy) this.callbacks.onBusyChange(this.state.busy)
    this.listeners.forEach((listener) => listener())
  }

  private cancelToken(token: string) {
    void this.call('extension_store_cancel', { token }).catch(this.callbacks.report)
  }

  activate() {
    this.active = true
    void this.load()
  }

  dispose() {
    this.active = false
    this.generation++
    this.pending = undefined
    this.closeReview()
  }

  closeReview = () => {
    this.operation++
    const token = this.state.review?.preview.token
    if (token) this.cancelToken(token)
    this.patch({
      review: undefined,
      trusted: false,
      busy: false,
      phase: undefined,
      preparingId: undefined
    })
  }

  setTrusted = (trusted: boolean) => {
    if (this.state.phase === 'review') this.patch({ trusted })
  }

  select = (entry?: StoreEntry) => {
    if (!this.active || this.state.busy) return
    const source = this.state.index?.source
    this.patch({ detail: entry && source ? { source, entry } : undefined, error: undefined })
  }

  load = (source = this.state.source): Promise<void> => {
    if (!this.active || this.state.phase === 'install') return Promise.resolve()
    if (this.pending?.source === source && this.pending) return this.pending.promise
    const switching = source !== this.state.source
    if (switching) this.closeReview()
    else if (this.state.busy) return Promise.resolve()
    const generation = ++this.generation
    this.patch({
      source,
      loading: true,
      error: undefined,
      notice: undefined,
      ...(switching ? { index: undefined, detail: undefined } : {})
    })
    const current = () => this.active && this.generation === generation
    const promise = (async () => {
      try {
        // Omit source only to read the backend preference, always bypassing its old cache key.
        const index = await this.call<StoreIndex>('extension_store_index', {
          ...(source ? { source } : {}),
          refresh: true
        })
        if (!current()) return
        if (source && index.source !== source) throw new Error('索引来源已变化，请重试')
        const entries = this.kind === 'plugin' ? index.plugins : index.adapters
        const selected = entries.find((entry) => entry.id === this.state.detail?.entry.id)
        this.patch({
          index,
          source: index.source,
          detail: selected ? { source: index.source, entry: selected } : undefined
        })
      } catch (cause) {
        this.callbacks.report(cause)
        if (current()) this.patch({ error: `商店索引加载失败：${this.callbacks.errorText(cause)}` })
      } finally {
        if (current()) {
          this.pending = undefined
          this.patch({ loading: false })
        }
      }
    })()
    this.pending = { source, promise }
    return promise
  }

  prepare = async (entry: StoreEntry, source: StoreSource) => {
    if (
      !this.active ||
      this.state.busy ||
      this.state.loading ||
      this.state.index?.source !== source ||
      !this.entries().includes(entry) ||
      unavailableReason(entry)
    )
      return
    const operation = ++this.operation
    const generation = this.generation
    const current = () =>
      this.active && operation === this.operation && generation === this.generation
    this.patch({
      busy: true,
      phase: 'prepare',
      preparingId: entry.id,
      error: undefined,
      notice: undefined,
      trusted: false
    })
    try {
      const preview = await this.call<StorePreview>('extension_store_prepare', {
        source,
        kind: this.kind,
        id: entry.id,
        version: entry.version
      })
      if (!current()) {
        this.cancelToken(preview.token)
        return
      }
      if (
        preview.kind !== this.kind ||
        preview.id !== entry.id ||
        preview.version !== entry.version
      ) {
        this.cancelToken(preview.token)
        throw new Error('安装预览与所选扩展不一致，请重试')
      }
      this.patch({ phase: 'review', preparingId: undefined, review: { source, preview } })
    } catch (cause) {
      this.callbacks.report(cause)
      if (current())
        this.patch({
          busy: false,
          phase: undefined,
          preparingId: undefined,
          error: `下载或校验安装包失败：${this.callbacks.errorText(cause)}`
        })
    }
  }

  install = async () => {
    const review = this.state.review
    if (!this.active || !review || !this.state.trusted || this.state.phase !== 'review') return
    if (review.source !== this.state.index?.source) return
    const operation = this.operation
    const current = () => this.active && this.operation === operation
    this.patch({ phase: 'install', error: undefined })
    try {
      await this.call('extension_store_install', { token: review.preview.token, confirmed: true })
    } catch (cause) {
      this.callbacks.report(cause)
      if (current()) {
        this.closeReview()
        this.patch({ error: `安装失败，请重试：${this.callbacks.errorText(cause)}` })
      }
      return
    }
    if (!current()) return
    // The install consumed the token; do not cancel it after success.
    this.patch({ review: undefined, phase: undefined, busy: false, trusted: false })
    void this.load(review.source)
    const upgraded = !!review.preview.installedVersion
    this.patch({
      notice:
        this.kind === 'adapter'
          ? `${upgraded ? '升级' : '安装'}成功，适配器已加载`
          : upgraded
            ? '升级成功，已保留原启用状态'
            : '安装成功，可在已安装中启用'
    })
    try {
      await this.callbacks.onInstalled()
    } catch (cause) {
      this.callbacks.report(cause)
      if (current())
        this.patch({ error: `安装已完成，刷新已安装列表失败：${this.callbacks.errorText(cause)}` })
    }
  }

  entries() {
    return (this.kind === 'plugin' ? this.state.index?.plugins : this.state.index?.adapters) ?? []
  }
}
