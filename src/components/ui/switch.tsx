import type { HTMLAttributes } from 'react'

import { cn } from '@/lib/utils'

type SwitchProps = Omit<HTMLAttributes<HTMLButtonElement>, 'onChange'> & {
  checked: boolean
  onCheckedChange: (checked: boolean) => void
}

export function Switch({
  checked,
  onCheckedChange,
  className,
  ...props
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={cn(
        'inline-flex h-6 w-11 items-center rounded-full border border-transparent p-0.5 transition outline-none focus:ring-2 focus:ring-[color-mix(in_oklab,var(--ring),transparent_70%)]',
        checked ? 'bg-[var(--primary)]' : 'bg-[var(--muted)]',
        className,
      )}
      onClick={() => onCheckedChange(!checked)}
      {...props}
    >
      <span
        className={cn(
          'block size-5 rounded-full bg-white transition',
          checked ? 'translate-x-5' : 'translate-x-0',
        )}
      />
    </button>
  )
}
