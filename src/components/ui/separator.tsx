import type { HTMLAttributes } from 'react'

import { cn } from '@/lib/utils'

export function Separator({ className, ...props }: HTMLAttributes<HTMLHRElement>) {
  return (
    <hr
      className={cn('border-0 border-t border-[var(--border)]', className)}
      {...props}
    />
  )
}
