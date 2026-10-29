import { useState } from 'react'
import { Button, MusicPageHeader } from '@nonsplayer/plugin-sdk'

export function HomePage() {
  const [count, setCount] = useState(0)
  return (
    <div className="flex flex-col gap-4">
      <MusicPageHeader title="示例插件" />
      <Button onClick={() => setCount((value) => value + 1)}>点击次数：{count}</Button>
    </div>
  )
}
