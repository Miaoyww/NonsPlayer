import type { ConfigSnapshot, PluginClient, PluginMenuSong, PluginTransfer } from '@app/plugin-sdk'
import { reportCaughtError } from '@app/plugin-sdk'
import { audioFormat, filename, LoginRequired, resolveResource, type Quality } from './protocol'
import { accountCredential, accountScope } from './account'

function songId(song: PluginMenuSong): number {
  if (song.source.kind !== 'online' || song.source.reference.source !== 'netease')
    throw new Error('只支持网易云歌曲')
  const id = Number(song.source.reference.id)
  if (!Number.isSafeInteger(id) || id <= 0) throw new Error('歌曲引用无效')
  return id
}

type Options = {
  quality: Quality
  minimumQuality: Quality
  allowFallback: boolean
  directory: string
}
export interface DownloadTask {
  id: number
  song: PluginMenuSong
  state:
    | 'waiting'
    | 'setup'
    | 'resolving'
    | 'running'
    | 'paused'
    | 'saving'
    | 'completed'
    | 'failed'
    | 'cancelled'
  options?: Options
  quality?: Quality
  bytes: number
  total?: number
  path?: string
  error?: string
  completedAt?: number
  transfer?: number
}
let client: PluginClient
let active = false
const inFlight = new Set<number>()

let tasks: DownloadTask[] = []
let view = { tasks, message: '' }
const adding = new Set<number>()
const listeners = new Set<() => void>()
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
const changed = () => {
  view = { tasks: [...tasks], message: view.message }
  listeners.forEach((fn) => fn())
}
export const subscribe = (fn: () => void) => {
  listeners.add(fn)
  return () => {
    listeners.delete(fn)
  }
}
export const snapshot = () => view
let persistence = Promise.resolve()
function persist() {
  const record = tasks.map(({ transfer: _, ...task }) => task)
  persistence = persistence
    .catch((cause) => reportCaughtError('netease-download.persistence.previous', cause))
    .then(() => client.call('storage.set', { key: 'tasks', value: record }))
    .then(() => {})
  return persistence
}
export async function activate(next: PluginClient) {
  client = next
  active = true
  try {
    const saved = await client.call<DownloadTask[] | null>('storage.get', { key: 'tasks' })
    tasks = (Array.isArray(saved) ? saved : []).slice(-32).map((task) => {
      if (task.state === 'running' || task.state === 'resolving' || task.state === 'saving')
        return { ...task, state: 'failed', error: '上次下载已中断，请重试', transfer: undefined }
      return { ...task, transfer: undefined }
    })
  } catch (error) {
    reportCaughtError('netease-download.restore', error)
    view.message = String(error)
  }
  changed()
  const timer = setInterval(() => {
    void pump()
  }, 2000)
  return () => {
    active = false
    clearInterval(timer)
    listeners.clear()
    adding.clear()
    inFlight.clear()
    tasks = []
    changed()
  }
}
export async function downloadSong(song: PluginMenuSong) {
  const id = songId(song)
  if (
    adding.has(id) ||
    tasks.some(
      (task) =>
        songId(task.song) === id &&
        ['waiting', 'setup', 'resolving', 'running', 'paused', 'saving'].includes(task.state)
    )
  ) {
    view.message = '该歌曲已在下载队列中'
    changed()
    client.openPage()
    return view.message
  }
  adding.add(id)
  try {
    if (tasks.length >= 32) {
      const i = tasks.findIndex((task) => ['completed', 'failed', 'cancelled'].includes(task.state))
      if (i < 0) throw new Error('下载队列已满（最多 32 首）')
      tasks.splice(i, 1)
    }
    const config = await client.call<ConfigSnapshot>('config.get')
    const options = config.values as Options
    tasks.push({
      id: Date.now() + Math.random(),
      song: {
        ...song,
        key: song.key.slice(0, 128),
        title: Array.from(song.title).slice(0, 80).join(''),
        artist: Array.from(song.artist).slice(0, 80).join(''),
        album: Array.from(song.album).slice(0, 80).join(''),
        cover: /^https?:/.test(song.cover) ? song.cover.slice(0, 512) : ''
      },
      state: 'waiting',
      options: { ...options },
      bytes: 0
    })
    const notice = `已加入下载队列：${song.title}`
    view.message = notice
    changed()
    await persist()
    if (!options.directory) client.openConfiguration()
    else client.openPage()
    void pump()
    return notice
  } finally {
    adding.delete(id)
  }
}
async function pump() {
  if (
    !active ||
    inFlight.size >= 2 ||
    tasks.some((task) => inFlight.has(task.id) && task.state !== 'paused')
  )
    return
  const task = tasks.find(
    (task) => !inFlight.has(task.id) && (task.state === 'waiting' || task.state === 'setup')
  )
  if (!task) return
  inFlight.add(task.id)
  const previous = JSON.stringify(task)
  let temporary: string | undefined
  let root: string | undefined
  try {
    const options = task.options!
    if (!options.directory) {
      const config = await client.call<ConfigSnapshot>('config.get')
      task.options = { ...options, directory: String(config.values.directory || '') }
    }
    root = task.options!.directory
    const roots = await client.call<{ id: string; writable: boolean }[]>('files.roots')
    const account = await accountCredential(client)
    if (!active || task.state === ('cancelled' as string) || task.state === ('paused' as string))
      return
    if (!root || !roots.some((r) => r.id === root && r.writable) || !account.cookie) {
      task.state = 'setup'
      task.error =
        !root || !roots.some((r) => r.id === root && r.writable)
          ? '请选择并授权保存目录'
          : '请先在 NonsPlayer 中登录网易云账号'
      if (JSON.stringify(task) !== previous) changed()
      return
    }
    task.state = 'resolving'
    task.error = undefined
    changed()
    await persist()
    const id = songId(task.song)
    const resource = await resolveResource(
      client,
      id,
      task.options!.quality,
      task.options!.allowFallback,
      task.options!.minimumQuality,
      account.cookie
    )
    if (!active || task.state === ('cancelled' as string) || task.state === ('paused' as string))
      return
    task.quality = resource.quality
    task.total = resource.size
    temporary = `${filename(task.song.title, task.song.artist, id)}-${Math.floor(task.id)}-${Date.now()}-${Math.floor(Math.random() * 1e9)}.part`
    const started = await client.call<{ id: number }>('transfers.start', {
      url: resource.url,
      root,
      path: temporary,
      maxBytes: resource.size
    })
    task.transfer = started.id
    if (task.state === ('cancelled' as string))
      await client.call('transfers.cancel', { id: started.id })
    else if (task.state === ('paused' as string))
      await client.call('transfers.pause', { id: started.id })
    else task.state = 'running'
    changed()
    await persist()
    for (;;) {
      await wait(500)
      if (!active) return
      const state = await client.call<PluginTransfer>('transfers.get', { id: task.transfer })
      task.bytes = state.bytes
      changed()
      if (state.state === 'running' || state.state === 'paused') continue
      if (state.state !== 'completed') throw new Error(state.error || '下载已取消')
      if (task.state === ('cancelled' as string)) throw new Error('下载已取消')
      if (state.bytes !== resource.size) throw new Error('实际文件大小与资源信息不符，请重试')
      break
    }
    task.state = 'saving'
    changed()
    const { handle } = await client.call<{ handle: number }>('files.open', {
      root,
      path: temporary,
      mode: 'read'
    })
    let format: ReturnType<typeof audioFormat>
    try {
      const { data } = await client.call<{ data: string }>('files.read', {
        handle,
        offset: 0,
        length: 32
      })
      format = audioFormat(Uint8Array.from(atob(data), (c) => c.charCodeAt(0)))
    } finally {
      await client.call('files.close', { handle })
    }
    if (!format) throw new Error('资源不是受支持的音频文件，已拒绝保存')
    if (task.state === ('cancelled' as string)) throw new Error('下载已取消')
    const target = `${filename(task.song.title, task.song.artist, id)}.${format}`
    const current = await accountScope(client)
    if (current.scope !== account.scope) throw new Error('下载期间账号已变化，请重试')
    await client.call('files.publish', { root, path: temporary, to: target })
    temporary = undefined
    task.path = target
    task.state = 'completed'
    task.completedAt = Date.now()
    changed()
  } catch (error) {
    reportCaughtError('netease-download.task', error)
    if (error instanceof LoginRequired) {
      view.message = '网易云登录已失效，请在 NonsPlayer 中重新登录后重试'
    }
    if (active && task.state !== 'cancelled') {
      task.state = 'failed'
      task.error = error instanceof Error ? error.message : String(error)
    }
    changed()
  } finally {
    if (temporary && root && active)
      await client
        .call('files.remove', { root, path: temporary })
        .catch((cause) => reportCaughtError('netease-download.cleanup', cause))
    if (active && JSON.stringify(task) !== previous)
      await persist().catch((error) => {
        reportCaughtError('netease-download.persist', error)
        view.message = `保存下载记录失败：${String(error)}`
        changed()
      })
    inFlight.delete(task.id)
    if (active && task.state !== 'setup') void pump()
  }
}
export async function cancelTask(id: number) {
  const task = tasks.find((task) => task.id === id)
  if (!task || ['completed', 'failed', 'cancelled', 'saving'].includes(task.state)) return
  task.state = 'cancelled'
  changed()
  if (task.transfer) await client.call('transfers.cancel', { id: task.transfer })
  await persist()
}
export async function retryTask(id: number) {
  const task = tasks.find((task) => task.id === id)
  if (!task || !['failed', 'cancelled', 'setup'].includes(task.state)) return
  const config = await client.call<ConfigSnapshot>('config.get')
  task.options = { ...config.values } as Options
  task.state = 'waiting'
  task.bytes = 0
  task.transfer = undefined
  task.error = undefined
  changed()
  await persist()
  if (!task.options.directory) client.openConfiguration()
  else client.openPage()
  void pump()
}
export function configure() {
  client.openConfiguration()
}
export async function pauseTask(id: number) {
  const task = tasks.find((task) => task.id === id)
  if (!task || !['waiting', 'setup', 'resolving', 'running'].includes(task.state)) return
  if (task.transfer) {
    const transfer = await client.call<PluginTransfer>('transfers.pause', { id: task.transfer })
    if (transfer && transfer.state !== 'paused') return
  }
  task.state = 'paused'
  changed()
  await persist()
  void pump()
}
export async function resumeTask(id: number) {
  const task = tasks.find((task) => task.id === id)
  if (!task || task.state !== 'paused') return
  if (task.transfer) {
    const state = await client.call<PluginTransfer>('transfers.get', { id: task.transfer })
    if (state.state === 'paused' || state.state === 'running') {
      await client.call('transfers.resume', { id: task.transfer })
      task.state = 'running'
    } else if (state.state === 'completed') task.state = 'running'
    else {
      task.state = 'failed'
      task.error = state.error || '传输已中断，请重试'
    }
  } else {
    task.state = 'waiting'
    task.bytes = 0
  }
  changed()
  await persist()
  void pump()
}
export async function deleteTask(id: number) {
  const task = tasks.find((task) => task.id === id)
  if (!task || task.state === 'saving' || task.state === 'completed') return
  await cancelTask(id)
  tasks = tasks.filter((task) => task.id !== id)
  changed()
  await persist()
}
export async function openDirectory() {
  const config = await client.call<ConfigSnapshot>('config.get')
  const root = String(config.values.directory || '')
  if (!root) {
    client.openConfiguration()
    return
  }
  await client.call('files.open-directory', { root })
}
