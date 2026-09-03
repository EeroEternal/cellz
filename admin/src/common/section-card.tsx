import { type ReactNode } from "react"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"
import { cn } from "@/lib/utils"

interface SectionCardProps {
  title: ReactNode
  description?: ReactNode
  children: ReactNode
  className?: string
  headerClassName?: string
  headerContentClassName?: string
  contentClassName?: string
  titleClassName?: string
  descriptionClassName?: string
  accentClassName?: string
  headerExtra?: ReactNode
}

export function SectionCard({
  title,
  description,
  children,
  className,
  headerClassName,
  headerContentClassName,
  contentClassName,
  titleClassName,
  descriptionClassName,
  accentClassName,
  headerExtra,
}: SectionCardProps) {
  return (
    <Card className={cn("border-border shadow-sm", className)}>
      <CardHeader className={cn("px-6 pb-4 pt-6", headerClassName)}>
        {accentClassName ? <div className={cn("mb-6 h-[2px] w-full rounded-full", accentClassName)} /> : null}
        <div
          className={cn(
            "flex flex-col gap-4",
            headerExtra ? "sm:flex-row sm:items-start sm:justify-between" : null,
            headerContentClassName,
          )}
        >
          <div>
            <CardTitle className={cn(titleClassName)}>{title}</CardTitle>
            {description ? <CardDescription className={cn("mt-1", descriptionClassName)}>{description}</CardDescription> : null}
          </div>
          {headerExtra}
        </div>
      </CardHeader>
      <CardContent className={contentClassName}>{children}</CardContent>
    </Card>
  )
}