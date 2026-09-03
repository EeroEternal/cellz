import { Check } from "lucide-react"
import { cn } from "@/lib/utils"

export interface WizardStepConfig {
  id: number
  label: string
}

export function WizardStepper({
  step,
  steps,
}: {
  step: number
  steps: WizardStepConfig[]
}) {
  return (
    <div className="flex flex-wrap items-center gap-2 text-xs font-medium text-muted-foreground">
      {steps.map((item, index) => (
        <div key={item.id} className="flex items-center gap-2">
          <span
            className={cn(
              "inline-flex h-7 w-7 items-center justify-center rounded-full border text-[11px]",
              step === item.id
                ? "border-primary bg-primary text-primary-foreground"
                : step > item.id
                  ? "border-primary/40 bg-primary/10 text-primary"
                  : "border-border bg-background"
            )}
          >
            {step > item.id ? <Check className="h-3.5 w-3.5" /> : item.id}
          </span>
          <span className={cn(step === item.id ? "text-foreground" : "")}>{item.label}</span>
          {index < steps.length - 1 ? <span className="mx-1 h-px w-6 bg-border sm:w-10" /> : null}
        </div>
      ))}
    </div>
  )
}
