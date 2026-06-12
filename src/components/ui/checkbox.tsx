import type { InputHTMLAttributes } from 'react'

import { cn } from '@/lib/utils'

export function Checkbox({
  className,
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      type="checkbox"
      className={cn(
        'size-4 rounded border border-[var(--border)] bg-[var(--input)] text-[var(--primary)] accent-[var(--primary)] outline-none focus:ring-2 focus:ring-[color-mix(in_oklab,var(--ring),transparent_72%)]',
        className,
      )}
      {...props}
    />
  )
}
