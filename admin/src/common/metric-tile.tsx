import { type ReactNode } from "react"

import { nestedMetricLabelClassName, nestedMetricTileClassName } from "@/common/card-height-presets"
import { cn } from "@/lib/utils"

interface MetricTileProps {
  label: ReactNode
  value: ReactNode
  className?: string
  labelClassName?: string
  valueClassName?: string
}

export function MetricTile({ label, value, className, labelClassName, valueClassName }: MetricTileProps) {
  return (
    <div className={cn(nestedMetricTileClassName, className)}>
      <div className={cn(nestedMetricLabelClassName, labelClassName)}>{label}</div>
      <div className={cn("metric-value text-metric mt-1", valueClassName)}>{value}</div>
    </div>
  )
}