import { useEffect, useSyncExternalStore } from 'react'
import { getPlayer, nativeCall, usePlayer } from '@/lib/player'
import { createVolumeSender } from './volume-sender'

const sender = createVolumeSender(
  (volume) => nativeCall('player_volume', { volume }),
  () => getPlayer().volume
)

export function changePlayerVolume(
  value: number,
  onError: (error: unknown) => void,
  immediate = false
) {
  sender.change(value, onError, immediate)
}
export function playerVolume() {
  return sender.getSnapshot() ?? getPlayer().volume
}

export function useVolumeControl(onError: (error: unknown) => void) {
  const { volume: confirmed } = usePlayer('volume')
  const optimistic = useSyncExternalStore(sender.subscribe, sender.getSnapshot)
  useEffect(() => sender.acknowledge(confirmed), [confirmed, optimistic])
  return {
    volume: optimistic ?? confirmed,
    changeVolume: (value: number, immediate = false) =>
      changePlayerVolume(value, onError, immediate),
    commitVolume: (value: number) => changePlayerVolume(value, onError, true)
  }
}
