import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

interface TwoPanelLayoutProps {
  left: ReactNode
  right: ReactNode
  className?: string
  /** When true, uses the canonical master–detail workspace grid on wide screens. */
  workspace?: boolean
}

export function TwoPanelLayout({ left, right, className, workspace = false }: TwoPanelLayoutProps) {
  return (
    <div
      className={cn(
        workspace
          ? "flex flex-1 min-h-0 flex-col gap-6 xl:grid xl:h-full xl:grid-cols-[minmax(40rem,1fr)_27.5rem] xl:items-stretch"
          : "flex flex-col xl:flex-row gap-6 flex-1 min-h-0",
        className
      )}
    >
      {left}
      {right}
    </div>
  )
}
