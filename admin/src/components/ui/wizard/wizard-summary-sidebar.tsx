import type { ReactNode } from "react"
import { cn } from "@/lib/utils"

export function WizardSummarySidebar({
  title,
  children,
  footer,
  className,
}: {
  title: string
  children: ReactNode
  footer?: ReactNode
  className?: string
}) {
  return (
    <aside
      className={cn(
        "flex h-full min-h-0 min-w-0 flex-col gap-4 overflow-hidden rounded-lg border border-border bg-card p-4",
        className
      )}
    >
      <h3 className="shrink-0 text-sm font-semibold">{title}</h3>
      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto text-sm">{children}</div>
      {footer ? (
        <div className="mt-auto flex shrink-0 flex-col gap-2 border-t border-border pt-4">{footer}</div>
      ) : null}
    </aside>
  )
}
