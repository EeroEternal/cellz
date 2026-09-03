import { type ReactNode } from "react"

import { cn } from "@/lib/utils"

interface PageActionBarProps {
  leading?: ReactNode
  trailing?: ReactNode
  className?: string
  leadingClassName?: string
  trailingClassName?: string
}

export function PageActionBar({
  leading,
  trailing,
  className,
  leadingClassName,
  trailingClassName,
}: PageActionBarProps) {
  return (
    <div className={cn("flex flex-col items-center justify-between gap-4 rounded-2xl border border-border/50 bg-card p-4 shadow-sm sm:flex-row", className)}>
      <div className={cn("flex items-center gap-4", leadingClassName)}>{leading}</div>
      <div className={cn("flex w-full items-center gap-4 sm:w-auto", trailingClassName)}>{trailing}</div>
    </div>
  )
}