import { X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { t } from "@/lib/i18n"
import { cn } from "@/lib/utils"

/** Top-right close control for multi-step create wizards (matches Route create dialog). */
export function WizardDialogCloseButton({
  onClose,
  className,
}: {
  onClose: () => void
  className?: string
}) {
  return (
    <Button
      type="button"
      variant="ghost"
      size="icon-sm"
      className={cn(
        "absolute right-4 top-4 z-50 shrink-0 text-muted-foreground hover:text-foreground",
        className
      )}
      onPointerDown={(event) => {
        event.preventDefault()
        event.stopPropagation()
        onClose()
      }}
      onClick={onClose}
      aria-label={t("common.close", "Close")}
      title={t("common.close", "Close")}
    >
      <X className="h-4 w-4" />
    </Button>
  )
}
