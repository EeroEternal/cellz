import { type ReactNode } from "react"

import {
  analyticsPanelCardClassName,
  analyticsPanelColumnClassName,
  analyticsPanelDescriptionClassName,
  analyticsPanelHeaderClassName,
} from "@/common/card-height-presets"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { cn } from "@/lib/utils"

interface InsightPanelProps {
  title: ReactNode
  description?: ReactNode
  children: ReactNode
  columnClassName?: string
  className?: string
  headerClassName?: string
  headerContentClassName?: string
  titleClassName?: string
  descriptionClassName?: string
  contentClassName?: string
}

export function InsightPanel({
  title,
  description,
  children,
  columnClassName,
  className,
  headerClassName,
  headerContentClassName,
  titleClassName,
  descriptionClassName,
  contentClassName,
}: InsightPanelProps) {
  return (
    <div className={cn(analyticsPanelColumnClassName, columnClassName)}>
      <Card className={cn(analyticsPanelCardClassName, className)}>
        <CardHeader className={cn(analyticsPanelHeaderClassName, headerClassName)}>
          <div className={cn("min-w-0 space-y-1.5", headerContentClassName)}>
            <CardTitle className={cn("whitespace-normal", titleClassName)}>{title}</CardTitle>
            {description ? <p className={cn("min-w-0 w-full", analyticsPanelDescriptionClassName, descriptionClassName)}>{description}</p> : null}
          </div>
        </CardHeader>
        <CardContent className={cn("flex min-h-0 flex-1 flex-col px-5 pb-5 pt-1", contentClassName)}>{children}</CardContent>
      </Card>
    </div>
  )
}