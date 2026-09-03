import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface DetailPanelProps {
  children: ReactNode
  className?: string
  contentClassName?: string
  scroll?: boolean
}

export function DetailPanel({ children, className, contentClassName, scroll = true }: DetailPanelProps) {
  return (
    <div className={cn("flex h-full min-w-0 flex-1 flex-col", className)}>
      <div
        className={cn(
          "flex h-full min-h-0 flex-1 flex-col rounded-lg border border-border bg-card p-6 scrollbar-hover",
          scroll ? "overflow-y-auto" : "overflow-hidden",
          contentClassName
        )}
      >
        {children}
      </div>
    </div>
  )
}
