import { reportCaughtError } from '@/lib/diagnostics'
import { ErrorNotice } from '@/components/error-notice'
import { hasPersonalPages } from '@/features/music/source-capabilities'
import { TextSkeleton } from '@/components/music/loading'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { Check, ChevronRight, QrCode, RefreshCw, UserPlus, LogOut } from 'lucide-react'
import { MusicPage, MusicPageHeader } from '@/components/music/music-page'
import { ActionButton } from '@/components/music/action-button'
import { musicClient } from '@/features/music/client'
import { sourcePreferences } from '@/features/music/source-preferences'
import { useMusicNavigation } from '@/features/workspace/music-navigation'
import { errorText } from '@/lib/player'
import { AccountAvatar } from './account-avatar'
import { SavedAccounts } from './saved-accounts'
import { useAccountAdapters, type AccountAdapter } from './use-account-adapters'
const noLoginMethods: { id: string; label: string }[] = []

export function AdapterLoginCard({
  adapter,
  onChanged,
  startLogin
}: {
  adapter: AccountAdapter
  onChanged: () => void
  startLogin: boolean
}) {
  const { source, displayName } = adapter.descriptor
  const methods = adapter.descriptor.loginMethods ?? noLoginMethods
  const [method, setMethod] = useState(methods[0]?.id)
  const [qr, setQr] = useState<{ key: string; image: string }>()
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [adding, setAdding] = useState(startLogin || !adapter.current)
  const initialLogin = useRef(startLogin || !adapter.current)
  const previousAccount = useRef(adapter.current?.reference.id)
  const generation = useRef(0)
  const intent = useRef<number | undefined>(undefined)
  const challenge = useRef<string | undefined>(undefined)
  const cancel = useCallback(() => {
    const key = challenge.current
    challenge.current = undefined
    if (key)
      void musicClient.cancelLogin(source, key).catch((cause) => {
        reportCaughtError('app/src/features/account/account-login-page.tsx', cause)
      })
  }, [source])
  const generate = useCallback(async () => {
    const serial = ++generation.current
    cancel()
    setAdding(true)
    setBusy(true)
    setQr(undefined)
    setMessage('正在生成二维码…')
    try {
      const nextIntent = hasPersonalPages(adapter.descriptor)
        ? await sourcePreferences.beginIntent()
        : undefined
      if (serial !== generation.current) return
      intent.current = nextIntent
      const result = await musicClient.account(source, { operation: 'beginLogin', method })
      if (result.type !== 'challenge') throw new Error('此来源暂未提供可展示的登录方式。')
      if (serial === generation.current) {
        challenge.current = result.data.key
        setQr(result.data)
        setMessage(
          `使用${methods.find((item) => item.id === method)?.label ?? displayName}，并确认登录。`
        )
      } else
        void musicClient.cancelLogin(source, result.data.key).catch((cause) => {
          reportCaughtError('app/src/features/account/account-login-page.tsx', cause)
        })
    } catch (cause) {
      reportCaughtError('app/src/features/account/account-login-page.tsx', cause)

      if (serial === generation.current) setMessage(errorText(cause))
    } finally {
      if (serial === generation.current) setBusy(false)
    }
  }, [source, displayName, method, methods, cancel])

  useEffect(() => {
    if (adapter.enabled && isTauri() && initialLogin.current) void generate()
    return () => {
      generation.current++
      cancel()
    }
  }, [adapter.enabled, generate, cancel])

  useEffect(() => {
    if (!qr || !adapter.enabled) return
    let disposed = false
    const serial = generation.current
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      try {
        const result = await musicClient.account(source, { operation: 'pollLogin', key: qr.key })
        if (disposed || serial !== generation.current) return
        if (result.type !== 'progress') throw new Error('登录状态响应无效。')
        setMessage(result.data.message)
        if (result.data.code === 803) {
          try {
            if (hasPersonalPages(adapter.descriptor))
              await sourcePreferences.use(source, intent.current!)
          } catch (cause) {
            reportCaughtError('app/src/features/account/account-login-page.tsx', cause)

            if (!disposed && serial === generation.current)
              setMessage(`登录成功，切换来源失败：${errorText(cause)}。可点击“使用此来源”重试。`)
          }
          if (disposed || serial !== generation.current) return
          challenge.current = undefined
          setQr(undefined)
          setAdding(false)
          onChanged()
          return
        }
        if (result.data.code === 800) {
          setQr(undefined)
          return
        }
      } catch (cause) {
        reportCaughtError('app/src/features/account/account-login-page.tsx', cause)

        if (!disposed && serial === generation.current) {
          setMessage(errorText(cause))
          setQr(undefined)
        }
        return
      }
      if (!disposed && serial === generation.current) timer = setTimeout(() => void poll(), 4000)
    }
    timer = setTimeout(() => void poll(), 4000)
    return () => {
      disposed = true
      clearTimeout(timer)
    }
  }, [qr, adapter.enabled, source, onChanged])

  // Selecting a saved account finishes any in-progress login without revoking credentials.
  useEffect(() => {
    if (adapter.current?.reference.id === previousAccount.current) return
    previousAccount.current = adapter.current?.reference.id
    if (challenge.current) return
    generation.current++
    setQr(undefined)
    setBusy(false)
    setAdding(!adapter.current)
    setMessage('')
  }, [adapter.current?.reference.id])

  return (
    <section
      aria-label={`${displayName}账号登录`}
      className="rounded-2xl border border-border bg-card p-5 text-card-foreground shadow-sm"
    >
      <header className="flex items-center justify-between gap-4">
        <div>
          <p className="text-sm text-muted-foreground">账号登录</p>
          <h2 className="mt-1 text-lg font-semibold">{displayName}</h2>
        </div>
        <QrCode className="size-6 text-muted-foreground" aria-hidden="true" />
      </header>
      {!adapter.enabled ? (
        <p className="py-6 text-sm text-muted-foreground">
          此适配器已停用。请在适配器设置中启用后登录，保存的账号仍会保留。
        </p>
      ) : adapter.current && !adding ? (
        <div className="flex flex-col items-center gap-3 py-6">
          <AccountAvatar avatar={adapter.current.avatar} className="size-14" />
          <p className="max-w-full truncate text-base font-semibold">
            {adapter.current.displayName}
          </p>
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Check className="size-4" aria-hidden="true" />
            当前账号已登录
          </p>
          {message && <ErrorNotice error={message} />}
          {hasPersonalPages(adapter.descriptor) && (
            <ActionButton
              variant="outline"
              disabled={busy}
              onClick={() => {
                setBusy(true)
                void sourcePreferences
                  .beginIntent()
                  .then((value) => sourcePreferences.use(source, value))
                  .then(() => setMessage(''))
                  .catch((cause) => {
                    reportCaughtError('app/src/features/account/account-login-page.tsx', cause)
                    return setMessage(`切换来源失败：${errorText(cause)}`)
                  })
                  .finally(() => setBusy(false))
              }}
            >
              使用此来源
            </ActionButton>
          )}
          <ActionButton
            variant="ghost"
            disabled={busy}
            onClick={() => {
              setBusy(true)
              void musicClient
                .account(source, { operation: 'logout' })
                .then((result) => {
                  if (result.type !== 'logout' || !result.data.localCleared)
                    throw new Error('退出当前账号失败，请重试。')
                  onChanged()
                })
                .catch((cause) => {
                  reportCaughtError('app/src/features/account/account-login-page.tsx', cause)
                  return setMessage(errorText(cause))
                })
                .finally(() => setBusy(false))
            }}
          >
            <LogOut aria-hidden="true" />
            退出账号
          </ActionButton>
          <ActionButton
            variant="secondary"
            onClick={() => void generate()}
            disabled={busy || !isTauri()}
          >
            <UserPlus aria-hidden="true" />
            添加另一个账号
          </ActionButton>
        </div>
      ) : (
        <div className="flex flex-col items-center gap-3 py-5">
          {methods.length > 1 && (
            <div role="group" aria-label="登录方式" className="flex gap-2">
              {methods.map((item) => (
                <ActionButton
                  key={item.id}
                  variant={method === item.id ? 'secondary' : 'outline'}
                  aria-pressed={method === item.id}
                  disabled={busy || !isTauri()}
                  onClick={() => {
                    if (method === item.id) return
                    generation.current++
                    cancel()
                    setQr(undefined)
                    setMessage('')
                    setMethod(item.id)
                  }}
                >
                  {item.label}
                </ActionButton>
              ))}
            </div>
          )}
          <div className="flex size-44 items-center justify-center rounded-xl border border-border bg-background p-2">
            {qr ? (
              <img
                src={qr.image}
                width={160}
                height={160}
                alt={`${displayName}登录二维码`}
                className="rounded-lg bg-white"
              />
            ) : (
              <QrCode className="size-12 text-muted-foreground/40" aria-hidden="true" />
            )}
          </div>
          <p role="status" className="text-center text-sm text-muted-foreground">
            {!isTauri() ? '请在桌面应用中登录。' : message || '生成二维码后扫码登录。'}
          </p>
          <ActionButton
            variant="secondary"
            disabled={busy || !isTauri()}
            onClick={() => void generate()}
          >
            <RefreshCw aria-hidden="true" />
            {busy ? '正在生成…' : qr ? '刷新二维码' : '生成二维码'}
          </ActionButton>
        </div>
      )}
      <SavedAccounts
        open
        switchPageSource={hasPersonalPages(adapter.descriptor)}
        source={source}
        sourceName={displayName}
        current={adapter.current?.reference}
        enabled={adapter.enabled}
        onChanged={onChanged}
      />
    </section>
  )
}

export default function AccountLoginPage() {
  const { page, navigate } = useMusicNavigation()
  const { adapters: allAdapters, loading, error, refresh } = useAccountAdapters()
  const adapters = allAdapters.filter((item) => hasPersonalPages(item.descriptor))
  const [source, setSource] = useState(page.view === 'accounts' ? page.query : '')
  const selected = adapters.find((item) => item.descriptor.source === source) ?? adapters[0]
  useEffect(() => {
    if (page.view === 'accounts' && page.query) setSource(page.query)
  }, [page.view, page.query])
  const onChanged = useCallback(() => {
    void refresh()
  }, [refresh])
  return (
    <MusicPage aria-label="音乐账号登录">
      <MusicPageHeader title="连接你的音乐">
        <p>选择音乐来源，登录或切换保存的账号。</p>
      </MusicPageHeader>
      {error && (
        <div className="mt-6 flex items-center gap-3 text-sm text-destructive">
          <ErrorNotice error={error} />
          <ActionButton variant="ghost" onClick={() => void refresh()}>
            重试
          </ActionButton>
        </div>
      )}
      {loading && !adapters.length ? (
        <TextSkeleton />
      ) : !adapters.length ? (
        <p className="py-6 text-sm text-muted-foreground">暂无支持账号登录的音乐适配器。</p>
      ) : (
        <div className="mt-6 grid w-full items-start gap-4 sm:grid-cols-[160px_minmax(0,1fr)]">
          <nav aria-label="音乐账号来源" className="grid gap-2">
            {adapters.map((item) => (
              <button
                type="button"
                key={item.descriptor.source}
                aria-pressed={selected?.descriptor.source === item.descriptor.source}
                onClick={() => setSource(item.descriptor.source)}
                className={`flex items-center gap-2 rounded-xl border p-3 text-left transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none ${selected?.descriptor.source === item.descriptor.source ? 'border-primary bg-primary/5' : 'border-border bg-card hover:bg-accent'}`}
              >
                <AccountAvatar avatar={item.current?.avatar} className="size-8" />
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-sm font-semibold">
                    {item.descriptor.displayName}
                  </span>
                  <span className="mt-1 block truncate text-xs text-muted-foreground">
                    {!item.enabled ? '已停用' : item.current ? item.current.displayName : '未登录'}
                  </span>
                </span>
                <ChevronRight
                  className="size-4 shrink-0 text-muted-foreground"
                  aria-hidden="true"
                />
              </button>
            ))}
          </nav>
          <div className="min-w-0">
            {selected && (
              <AdapterLoginCard
                key={`${selected.descriptor.source}:${page.view}:${page.query}`}
                adapter={selected}
                onChanged={onChanged}
                startLogin={page.view === 'accounts'}
              />
            )}
            {adapters.some((item) => item.enabled && item.current) && (
              <div className="mt-5 flex justify-end">
                <ActionButton variant="ghost" onClick={() => navigate('library')}>
                  前往音乐库
                  <ChevronRight aria-hidden="true" />
                </ActionButton>
              </div>
            )}
          </div>
        </div>
      )}
    </MusicPage>
  )
}
