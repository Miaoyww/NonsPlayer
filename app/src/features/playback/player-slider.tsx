import { Slider, type SliderProps } from '@/components/ui/slider'
import { cn } from 'cn'

type PlayerSliderProps = Omit<SliderProps, 'value' | 'defaultValue' | 'className'> & {
  value: number
  className?: string
}

export function PlayerSlider({ className, ...props }: PlayerSliderProps) {
  return <Slider {...props} className={cn('player-slider', className)} />
}
