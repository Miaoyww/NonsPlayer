/** Windows-only capabilities, explicitly authorized per plugin. */
export interface PluginProcessOptions {
  program: string
  args?: string[]
  cwd?: string
}
export interface PluginProcessStatus {
  id: number
  pid: number
  state: 'running' | 'exited' | 'terminated'
  exitCode: number | null
}
export interface PluginProcessOutput {
  data: string
  cursor: number
  nextCursor: number
  total: number
  dropped: number
  eof: boolean
  error: string | null
}
export interface PluginProcesses {
  spawn(options: PluginProcessOptions): Promise<{ id: number; pid: number }>
  get(id: number): Promise<PluginProcessStatus>
  read(
    id: number,
    stream: 'stdout' | 'stderr',
    cursor?: number,
    maxBytes?: number
  ): Promise<PluginProcessOutput>
  write(id: number, data: string): Promise<{ queuedBytes: number }>
  closeStdin(id: number): Promise<void>
  terminate(id: number): Promise<void>
  dispose(id: number): Promise<void>
}
export interface PluginWindowUpdate {
  title?: string
  width?: number
  height?: number
  x?: number
  y?: number
  alwaysOnTop?: boolean
  ignoreCursorEvents?: boolean
}
export interface PluginWindowOptions extends PluginWindowUpdate {
  transparent?: boolean
  decorations?: boolean
}
export interface PluginWindows {
  open(id: string, options?: PluginWindowOptions): Promise<{ id: string; label: string }>
  update(id: string, options: PluginWindowUpdate): Promise<void>
  show(id: string): Promise<void>
  hide(id: string): Promise<void>
  close(id: string): Promise<void>
  startDragging(id: string): Promise<void>
}
