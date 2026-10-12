import FastPriorityQueue from 'fastpriorityqueue'

export interface AuxiliaryLine {
  start: number
  text: string
}

interface Bucket {
  time: number
  lines: number[]
  entries: number[]
  lineCursor: number
  entryCursor: number
  previous?: Bucket
  next?: Bucket
  active: boolean
  version: number
}
interface Pair {
  left: Bucket
  right: Bucket
  leftVersion: number
  rightVersion: number
  line: number
  entry: number
  distance: number
}

// Globally nearest pairs are adjacent opposing time buckets. Grouping ties and
// keeping only adjacent candidates avoids allocating a quadratic pair matrix.
export function matchAuxiliary(startTimes: number[], entries: AuxiliaryLine[], tolerance = 500) {
  const matched: (string | undefined)[] = Array(startTimes.length)
  if (!startTimes.length || !entries.length) return matched
  const groups = new Map<number, Bucket>()
  const group = (time: number) => {
    let bucket = groups.get(time)
    if (!bucket) {
      bucket = {
        time,
        lines: [],
        entries: [],
        lineCursor: 0,
        entryCursor: 0,
        active: true,
        version: 0
      }
      groups.set(time, bucket)
    }
    return bucket
  }
  startTimes.forEach((time, index) => {
    if (Number.isFinite(time)) group(time).lines.push(index)
  })
  entries.forEach((entry, index) => {
    if (Number.isFinite(entry.start)) group(entry.start).entries.push(index)
  })
  const buckets = [...groups.values()].sort((a, b) => a.time - b.time)
  const remaining = buckets.filter((bucket) => {
    while (bucket.lineCursor < bucket.lines.length && bucket.entryCursor < bucket.entries.length) {
      matched[bucket.lines[bucket.lineCursor++]] =
        entries[bucket.entries[bucket.entryCursor++]].text
    }
    return bucket.lineCursor < bucket.lines.length || bucket.entryCursor < bucket.entries.length
  })
  remaining.forEach((bucket, index) => {
    bucket.previous = remaining[index - 1]
    bucket.next = remaining[index + 1]
  })
  const queue = new FastPriorityQueue<Pair>((a, b) =>
    a.distance !== b.distance
      ? a.distance < b.distance
      : a.line !== b.line
        ? a.line < b.line
        : a.entry < b.entry
  )
  const add = (left?: Bucket, right?: Bucket) => {
    if (!left || !right || !left.active || !right.active || right.time - left.time > tolerance)
      return
    const leftIsLine = left.lineCursor < left.lines.length
    const rightIsLine = right.lineCursor < right.lines.length
    if (leftIsLine === rightIsLine) return
    const lineBucket = leftIsLine ? left : right
    const entryBucket = leftIsLine ? right : left
    queue.add({
      left,
      right,
      leftVersion: left.version,
      rightVersion: right.version,
      line: lineBucket.lines[lineBucket.lineCursor],
      entry: entryBucket.entries[entryBucket.entryCursor],
      distance: right.time - left.time
    })
  }
  remaining.forEach((bucket) => add(bucket, bucket.next))
  const consume = (bucket: Bucket) => {
    bucket.version++
    if (bucket.lineCursor < bucket.lines.length) bucket.lineCursor++
    else bucket.entryCursor++
    if (bucket.lineCursor === bucket.lines.length && bucket.entryCursor === bucket.entries.length) {
      bucket.active = false
      if (bucket.previous) bucket.previous.next = bucket.next
      if (bucket.next) bucket.next.previous = bucket.previous
    }
  }
  while (!queue.isEmpty()) {
    const pair = queue.poll()!
    if (
      !pair.left.active ||
      !pair.right.active ||
      pair.left.next !== pair.right ||
      pair.left.version !== pair.leftVersion ||
      pair.right.version !== pair.rightVersion
    )
      continue
    matched[pair.line] = entries[pair.entry].text
    consume(pair.left)
    consume(pair.right)
    for (const bucket of [pair.left, pair.right]) {
      if (bucket.active) {
        add(bucket.previous, bucket)
        add(bucket, bucket.next)
      } else add(bucket.previous, bucket.next)
    }
  }
  return matched
}
