import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react'
import { Download, Package, RefreshCw, Search, Store } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue
} from '@/components/ui/select'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle
} from '@/components/ui/dialog'
import { ErrorNotice } from '@/components/error-notice'
import { TextSkeleton } from '@/components/music/loading'
import { InfiniteLoad } from '@/components/music/infinite-load'
import { nativeCall, errorText } from '@/lib/player'
import { reportCaughtError } from '@/lib/diagnostics'
import {
  ExtensionStoreController,
  permissionLines,
  searchEntries,
  unavailableReason,
  type StoreEntry,
  type StoreKind
} from './extension-store-state'

export type { StoreEntry } from './extension-store-state'

export interface ExtensionStoreProps {
  kind: StoreKind
  busy?: boolean
  onInstalled: () => void | Promise<void>
  installed?: readonly { id: string; version?: string }[]
  onBusyChange?: (busy: boolean) => void
}

function EntryMetadata({ entry }: { entry: StoreEntry }) {
  const permissions = permissionLines(entry)
  const notes = entry.releaseNotes || entry.release?.description || entry.release?.notes
  return (
    <div className="space-y-4 text-sm">
      <p className="whitespace-pre-wrap break-words text-muted-foreground">
        {entry.description || '暂无描述'}
      </p>
      <dl className="grid min-w-0 grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-2 break-words">
        <dt className="text-muted-foreground">ID</dt>
        <dd>{entry.id}</dd>
        <dt className="text-muted-foreground">作者</dt>
        <dd>{entry.authors?.map((author) => author.name).join('、') || '未提供'}</dd>
        <dt className="text-muted-foreground">许可证</dt>
        <dd>{entry.license || '未提供'}</dd>
        <dt className="text-muted-foreground">版本</dt>
        <dd>{entry.version}</dd>
        {entry.installedVersion && (
          <>
            <dt className="text-muted-foreground">已安装</dt>
            <dd>{entry.installedVersion}</dd>
          </>
        )}
        <dt className="text-muted-foreground">仓库</dt>
        <dd className="break-all">{entry.repository || '未提供'}</dd>
        <dt className="text-muted-foreground">安装包来源</dt>
        <dd>GitHub Release</dd>
      </dl>
      <div>
        <h4 className="font-medium">权限与范围</h4>
        {permissions.length ? (
          <ul className="mt-2 list-disc space-y-1 pl-5 break-words">
            {permissions.map((permission) => (
              <li key={permission}>{permission}</li>
            ))}
          </ul>
        ) : (
          <p className="mt-1 text-muted-foreground">未声明权限</p>
        )}
      </div>
      {entry.labels?.length ? (
        <p className="break-words text-xs text-muted-foreground">标签：{entry.labels.join('、')}</p>
      ) : null}
      {notes && (
        <div>
          <h4 className="font-medium">更新说明</h4>
          <p className="mt-2 whitespace-pre-wrap break-words text-muted-foreground">{notes}</p>
        </div>
      )}
    </div>
  )
}

export function ExtensionStore({
  kind,
  busy = false,
  onInstalled,
  onBusyChange
}: ExtensionStoreProps) {
  const callbacks = useRef({ onInstalled, onBusyChange })
  callbacks.current = { onInstalled, onBusyChange }
  const controller = useMemo(
    () =>
      new ExtensionStoreController(kind, nativeCall, {
        onInstalled: () => callbacks.current.onInstalled(),
        onBusyChange: (value) => callbacks.current.onBusyChange?.(value),
        report: (cause) => reportCaughtError('settings.extension-store', cause),
        errorText
      }),
    [kind]
  )
  const state = useSyncExternalStore(controller.subscribe, controller.getSnapshot)
  const [query, setQuery] = useState('')
  const [limit, setLimit] = useState(24)
  const noun = kind === 'plugin' ? '插件' : '适配器'
  useEffect(() => {
    controller.activate()
    return () => controller.dispose()
  }, [controller])
  useEffect(() => setLimit(24), [query, state.source, kind])
  const entries = searchEntries(controller.entries(), query)
  const detail = state.detail
  const preview = state.review?.preview
  const action = preview?.installedVersion ? '升级' : '安装'
  const disabled = busy || state.busy || state.loading

  function installButton(entry: StoreEntry, source = state.index?.source) {
    const reason = unavailableReason(entry)
    return (
      <div className="space-y-1">
        <Button
          size="sm"
          disabled={disabled || !!reason}
          onClick={() => {
            if (source) void controller.prepare(entry, source)
          }}
        >
          <Download aria-hidden="true" />
          {reason
            ? entry.bundled
              ? '已随包安装'
              : entry.distribution === 'source'
                ? '暂不可安装'
                : entry.installedVersion && !entry.updateAvailable
                  ? '已安装'
                  : '不可安装'
            : state.phase === 'prepare' && state.preparingId === entry.id
              ? '正在下载安装包…'
              : entry.updateAvailable
                ? '更新'
                : '安装'}
        </Button>
        {reason && <p className="text-xs leading-normal text-muted-foreground">{reason}</p>}
      </div>
    )
  }

  return (
    <section
      aria-label={`${noun}商店`}
      className="min-w-0 space-y-4 rounded-xl border border-border p-4"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex items-start gap-3">
          <Store aria-hidden="true" className="mt-0.5 size-5 shrink-0 text-muted-foreground" />
          <div>
            <h3 className="text-sm font-semibold">{noun}商店</h3>
            <p className="mt-1 text-xs text-muted-foreground">安装包来源：GitHub Release</p>
          </div>
        </div>
        <Button
          size="sm"
          variant="outline"
          disabled={disabled}
          onClick={() => void controller.load()}
        >
          <RefreshCw aria-hidden="true" />
          {state.loading ? '正在检查…' : '检查更新'}
        </Button>
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <label className="min-w-0 flex-1 basis-48 space-y-1.5">
          <span className="text-xs font-medium">搜索{noun}</span>
          <div className="relative">
            <Search
              aria-hidden="true"
              className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted-foreground"
            />
            <Input
              className="pl-9"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="名称、ID、描述或标签"
            />
          </div>
        </label>
        <div className="space-y-1.5">
          <span id={`store-source-${kind}`} className="text-xs font-medium">
            索引来源
          </span>
          <Select
            value={state.source ?? 'cdn'}
            disabled={state.phase === 'install'}
            onValueChange={(value) => {
              if (value === 'cdn' || value === 'github') void controller.load(value)
            }}
          >
            <SelectTrigger aria-labelledby={`store-source-${kind}`}>
              <SelectValue>{state.source === 'github' ? 'GitHub meta' : 'CDN（默认）'}</SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="cdn">CDN（默认）</SelectItem>
              <SelectItem value="github">GitHub meta</SelectItem>
            </SelectContent>
          </Select>
        </div>
      </div>
      {state.index && (
        <p className="break-all text-xs leading-normal text-muted-foreground">
          当前索引：{state.index.source === 'github' ? 'GitHub meta' : 'CDN'} · {state.index.url}
          {state.index.generatedAt ? ` · ${state.index.generatedAt}` : ''}
        </p>
      )}
      <ErrorNotice error={state.error} />
      {state.error && !state.review && (
        <Button
          size="sm"
          variant="outline"
          disabled={disabled}
          onClick={() => void controller.load()}
        >
          重试加载索引
        </Button>
      )}
      {state.notice && (
        <p role="status" className="text-sm">
          {state.notice}
        </p>
      )}
      <div aria-busy={state.loading} className="space-y-3">
        {state.loading && !state.index && <TextSkeleton />}
        {entries.slice(0, limit).map((entry) => (
          <article
            key={entry.id}
            className="flex flex-wrap items-start justify-between gap-3 rounded-xl border border-border p-3"
          >
            <div className="flex min-w-0 flex-1 items-start gap-3">
              <Package
                aria-hidden="true"
                className="mt-0.5 size-5 shrink-0 text-muted-foreground"
              />
              <div className="min-w-0 space-y-1">
                <Button
                  variant="link"
                  size="sm"
                  className="h-auto max-w-full justify-start p-0 whitespace-normal text-left"
                  disabled={state.busy}
                  onClick={() => controller.select(entry)}
                >
                  {entry.name}
                </Button>
                <p className="break-all text-xs text-muted-foreground">
                  {entry.id} · {entry.version}
                  {entry.installedVersion ? ` · 已安装 ${entry.installedVersion}` : ''}
                  {entry.updateAvailable ? ' · 有更新' : ''}
                </p>
                <p className="break-words text-xs leading-normal text-muted-foreground">
                  {entry.description}
                </p>
              </div>
            </div>
            {installButton(entry)}
          </article>
        ))}
        {!state.loading && state.index && !entries.length && (
          <p className="text-sm text-muted-foreground">
            {query ? '没有匹配的扩展' : `暂无${noun}`}
          </p>
        )}
        {entries.length > limit && (
          <InfiniteLoad more busy={state.loading} onLoad={() => setLimit((value) => value + 24)} />
        )}
      </div>
      <Dialog
        open={!!detail && !preview}
        onOpenChange={(open) => {
          if (!open && !state.busy) controller.select()
        }}
      >
        <DialogContent className="max-h-[85dvh] overflow-y-auto sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>{detail?.entry.name || `${noun}详情`}</DialogTitle>
            <DialogDescription>
              索引来源：{detail?.source === 'github' ? 'GitHub meta' : 'CDN'}；安装包来自 GitHub
              Release。
            </DialogDescription>
          </DialogHeader>
          {detail && (
            <>
              <EntryMetadata entry={detail.entry} />
              <ErrorNotice error={state.error} />
              <DialogFooter>{installButton(detail.entry, detail.source)}</DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
      <Dialog
        open={!!preview}
        onOpenChange={(open) => {
          if (!open && state.phase !== 'install') controller.closeReview()
        }}
      >
        <DialogContent
          showCloseButton={state.phase !== 'install'}
          className="max-h-[85dvh] overflow-y-auto sm:max-w-lg"
        >
          <DialogHeader>
            <DialogTitle>
              确认{action}
              {noun}
            </DialogTitle>
            <DialogDescription>
              {preview?.name} · {preview?.version}。请核对权限并确认信任作者。
            </DialogDescription>
          </DialogHeader>
          {preview && (
            <>
              <p className="break-all text-xs text-muted-foreground">
                {preview.id}
                {preview.installedVersion ? ` · 当前版本 ${preview.installedVersion}` : ''} · 索引：
                {state.review?.source === 'github' ? 'GitHub meta' : 'CDN'} · 安装包：GitHub Release
              </p>
              <div>
                <h4 className="text-sm font-medium">安装包请求的权限</h4>
                {preview.permissions.length ? (
                  <ul className="mt-2 list-disc space-y-1 pl-5 text-sm break-words">
                    {preview.permissions.map((permission, index) => (
                      <li key={index}>{permission}</li>
                    ))}
                  </ul>
                ) : (
                  <p className="mt-1 text-sm text-muted-foreground">未请求额外权限</p>
                )}
              </div>
              {preview.warnings.map((warning, index) => (
                <p key={index} className="whitespace-pre-wrap break-words text-sm text-destructive">
                  {warning}
                </p>
              ))}
              <label className="flex cursor-pointer items-start gap-2 text-sm leading-normal">
                <input
                  type="checkbox"
                  className="mt-0.5 size-4 shrink-0 accent-primary"
                  checked={state.trusted}
                  disabled={state.phase === 'install'}
                  onChange={(event) => controller.setTrusted(event.target.checked)}
                />
                我已了解以上权限并信任此扩展及其作者，同意{action}。
              </label>
              <ErrorNotice error={state.error} />
              <DialogFooter>
                <Button
                  variant="outline"
                  disabled={state.phase === 'install'}
                  onClick={controller.closeReview}
                >
                  取消
                </Button>
                <Button
                  disabled={!state.trusted || state.phase !== 'review'}
                  onClick={() => void controller.install()}
                >
                  {state.phase === 'install' ? `正在${action}…` : `确认${action}`}
                </Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>
    </section>
  )
}
