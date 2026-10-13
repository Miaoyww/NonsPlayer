import { getCoreRowModel, useReactTable, type ColumnSizingState } from '@tanstack/react-table'
import { useLayoutEffect, useMemo, useRef, useState } from 'react'

interface ColumnSpec {
  id: string
  label: string
  size: number
  minSize: number
}
const data: never[] = []

export function useTrackColumnSizing(specs: ColumnSpec[]) {
  const container = useRef<HTMLDivElement>(null)
  const [width, setWidth] = useState(900)
  const [custom, setCustom] = useState<ColumnSizingState>({})
  useLayoutEffect(() => {
    const element = container.current
    if (!element) return
    const measure = () => {
      const style = element.ownerDocument.defaultView!.getComputedStyle(element)
      const contentWidth =
        element.clientWidth -
        (parseFloat(style.paddingLeft) || 0) -
        (parseFloat(style.paddingRight) || 0)
      if (contentWidth > 0) setWidth(contentWidth)
    }
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(element)
    return () => observer.disconnect()
  }, [])
  const columns = useMemo(() => {
    const fixed = specs
      .filter((column) => column.id !== 'title' && column.id !== 'album')
      .reduce((total, column) => total + column.size, 0)
    const album = specs.some((column) => column.id === 'album') ? Math.max(100, width * 0.22) : 0
    return specs.map((column) => ({
      ...column,
      header: column.label,
      size:
        column.id === 'title'
          ? Math.max(column.minSize, width - fixed - album)
          : column.id === 'album'
            ? album
            : column.size
    }))
  }, [width, specs])
  const defaults = Object.fromEntries(columns.map((column) => [column.id, column.size]))
  const sizing = { ...defaults, ...custom }
  const table = useReactTable({
    data,
    columns,
    getCoreRowModel: getCoreRowModel(),
    columnResizeMode: 'onChange',
    state: { columnSizing: sizing },
    onColumnSizingChange: (update) => {
      const proposed = typeof update === 'function' ? update(sizing) : update
      const index = columns.findIndex((column) => proposed[column.id] !== sizing[column.id])
      if (index < 0 || index === columns.length - 1) return
      const left = columns[index],
        right = columns[index + 1]
      const total = sizing[left.id] + sizing[right.id]
      const size = Math.max(left.minSize, Math.min(total - right.minSize, proposed[left.id]))
      setCustom({ ...sizing, [left.id]: size, [right.id]: total - size })
    }
  })
  const headers = table.getFlatHeaders()
  function handle(id: string) {
    const index = columns.findIndex((column) => column.id === id)
    if (index < 0 || index === columns.length - 1) return null
    const header = headers[index]
    return (
      <div
        role="separator"
        tabIndex={0}
        aria-orientation="vertical"
        aria-label={`调整${columns[index].label}列宽`}
        aria-valuemin={columns[index].minSize}
        aria-valuemax={sizing[id] + sizing[columns[index + 1].id] - columns[index + 1].minSize}
        aria-valuenow={Math.round(sizing[id])}
        className="track-column-resizer"
        data-resizing={header.column.getIsResizing()}
        title="双击恢复默认"
        onMouseDown={header.getResizeHandler()}
        onTouchStart={header.getResizeHandler()}
        onDoubleClick={() => setCustom({})}
        onKeyDown={(event) => {
          if (event.key === 'Home') {
            event.preventDefault()
            setCustom({})
          } else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
            event.preventDefault()
            table.setColumnSizing({
              ...sizing,
              [id]: sizing[id] + (event.key === 'ArrowRight' ? 1 : -1) * (event.shiftKey ? 32 : 8)
            })
          }
        }}
      />
    )
  }
  return {
    container,
    columns: headers.map((header) => ({ id: header.id, size: header.getSize() })),
    total: table.getTotalSize(),
    handle
  }
}
