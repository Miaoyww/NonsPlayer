import { reportCaughtError } from '@/lib/diagnostics'
import { useCallback, useEffect, useRef, useState } from 'react'
import { isTauri } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { musicClient } from '@/features/music/client'
import type { AccountRecord, SourceDescriptor } from '@/features/music/types'
import { errorText, nativeCall } from '@/lib/player'

export interface AccountAdapter {
  descriptor: SourceDescriptor
  enabled: boolean
  current: AccountRecord | null
}

// Account metadata is live state, shared by the menu and workspace login page.
export function useAccountAdapters() {
  const [adapters, setAdapters] = useState<AccountAdapter[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')
  const generation = useRef(0)
  const mounted = useRef(false)
  const refresh = useCallback(async () => {
    const serial = ++generation.current
    setLoading(true)
    setError('')
    try {
      if (!isTauri()) throw new Error('请在桌面应用中登录音乐账号。')
      const statuses = await nativeCall<Omit<AccountAdapter, 'current'>[]>('adapter_list')
      const result = await Promise.all(
        statuses
          .filter((item) => item.descriptor.capabilities.includes('account'))
          .map(async (item) => ({
            ...item,
            current: await musicClient.currentAccount(item.descriptor.source)
          }))
      )
      if (mounted.current && serial === generation.current) setAdapters(result)
    } catch (cause) {
      reportCaughtError('app/src/features/account/use-account-adapters.ts', cause)

      if (mounted.current && serial === generation.current) setError(errorText(cause))
    } finally {
      if (mounted.current && serial === generation.current) setLoading(false)
    }
  }, [])
  useEffect(() => {
    mounted.current = true
    void refresh()
    let disposed = false
    const stops: (() => void)[] = []
    if (isTauri())
      for (const event of ['music-account-changed', 'adapters-changed'])
        void listen(event, () => {
          void refresh()
        })
          .then((stop) => {
            if (disposed) stop()
            else stops.push(stop)
          })
          .catch((cause) => {
            reportCaughtError('app/src/features/account/use-account-adapters.ts', cause)

            if (!disposed) setError(errorText(cause))
          })
    return () => {
      disposed = true
      mounted.current = false
      generation.current++
      stops.forEach((stop) => stop())
    }
  }, [refresh])
  return { adapters, loading, error, refresh }
}
