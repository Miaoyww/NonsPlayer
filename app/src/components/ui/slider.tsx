import { Slider as SliderPrimitive } from '@base-ui/react/slider'
import { cn } from 'cn'
import type { CSSProperties } from 'react'

type SliderProps = SliderPrimitive.Root.Props & {
  thumbProps?: SliderPrimitive.Thumb.Props
  /** Color of the filled track. Accepts any CSS color, including CSS variables. */
  rangeColor?: CSSProperties['color']
}

function Slider({
  className,
  defaultValue,
  value,
  min = 0,
  max = 100,
  rangeColor = 'var(--slider-fill, var(--foreground))',
  thumbProps,
  'aria-label': ariaLabel,
  'aria-valuetext': ariaValueText,
  ...props
}: SliderProps) {
  const values = value ?? defaultValue ?? min
  const thumbCount = Array.isArray(values) ? values.length : 1

  return (
    <SliderPrimitive.Root
      className={cn('group/slider data-horizontal:w-full data-vertical:h-full', className)}
      aria-label={ariaLabel}
      data-slot="slider"
      defaultValue={defaultValue}
      value={value}
      min={min}
      max={max}
      thumbAlignment="center"
      {...props}
    >
      <SliderPrimitive.Control className="relative flex min-h-6 w-full touch-none items-center select-none data-disabled:opacity-50 data-vertical:h-full data-vertical:w-auto data-vertical:flex-col">
        <SliderPrimitive.Track
          data-slot="slider-track"
          style={{ backgroundColor: 'rgb(160 160 160 / 45%)' }}
          className="relative grow overflow-hidden rounded-4xl bg-muted select-none data-horizontal:h-1 data-horizontal:w-full data-vertical:h-full data-vertical:w-1"
        >
          <SliderPrimitive.Indicator
            data-slot="slider-range"
            className="select-none data-horizontal:h-full data-vertical:w-full"
            style={{ backgroundColor: rangeColor }}
          />
        </SliderPrimitive.Track>
        {Array.from({ length: thumbCount }, (_, index) => (
          <SliderPrimitive.Thumb
            getAriaLabel={ariaLabel ? () => ariaLabel : undefined}
            aria-valuetext={ariaValueText}
            {...thumbProps}
            data-slot="slider-thumb"
            key={index}
            index={index}
            className="block size-4 shrink-0 rounded-4xl border opacity-0 shadow-sm ring-ring/50 transition-[opacity,box-shadow] select-none group-hover/slider:opacity-100 data-dragging:opacity-100 focus-within:opacity-100 focus-within:ring-4 hover:ring-4 focus-visible:ring-4 focus-visible:outline-hidden disabled:pointer-events-none motion-reduce:transition-none"
            style={{ borderColor: rangeColor, backgroundColor: rangeColor }}
          />
        ))}
      </SliderPrimitive.Control>
    </SliderPrimitive.Root>
  )
}

export { Slider, type SliderProps }
