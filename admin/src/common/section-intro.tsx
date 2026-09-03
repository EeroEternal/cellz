import { type ReactNode } from "react"

import { cn } from "@/lib/utils"

interface SectionIntroProps {
  title: ReactNode
  description?: ReactNode
  className?: string
  titleClassName?: string
  descriptionClassName?: string
}

export function SectionIntro({ title, description, className, titleClassName, descriptionClassName }: SectionIntroProps) {
  return (
    <div className={cn("space-y-1", className)}>
      <h2 className={cn("text-section-title text-foreground", titleClassName)}>{title}</h2>
      {description ? <p className={cn("text-sm text-muted-foreground", descriptionClassName)}>{description}</p> : null}
    </div>
  )
}