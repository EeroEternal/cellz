import { type ReactNode } from "react"

import { cn } from "@/lib/utils"

interface InsightListItemProps {
  children: ReactNode
  className?: string
}

export function InsightListItem({ children, className }: InsightListItemProps) {
  return <div className={cn("rounded-lg border border-border bg-muted/30 px-3 py-2.5", className)}>{children}</div>
}