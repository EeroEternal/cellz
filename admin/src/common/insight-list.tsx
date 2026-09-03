import { type ReactNode } from "react"

import { ScrollableCardContent } from "@/components/ui/scrollable-card-content"
import { cn } from "@/lib/utils"

interface InsightListProps {
  children: ReactNode
  viewportClassName?: string
  contentClassName?: string
}

export function InsightList({ children, viewportClassName, contentClassName }: InsightListProps) {
  return (
    <ScrollableCardContent
      viewportClassName={cn("flex-1", viewportClassName)}
      contentClassName={cn("space-y-2", contentClassName)}
    >
      {children}
    </ScrollableCardContent>
  )
}