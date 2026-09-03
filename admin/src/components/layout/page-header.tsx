import { cn } from "@/lib/utils"

interface PageHeaderProps {
    title?: string
    action?: React.ReactNode
    className?: string
}

export function PageHeader({ title, action, className }: PageHeaderProps) {
    return (
        <div className={cn("mx-auto mb-6 flex w-full max-w-[1400px] items-center justify-between gap-3", className)}>
            {title ? (
                <h1 className="min-w-0 break-words text-page-title tracking-tight text-foreground">
                    {title}
                </h1>
            ) : (
                <div className="min-w-0" />
            )}
            {action ? (
                <div className="flex shrink-0 flex-nowrap items-center justify-end gap-2">
                    {action}
                </div>
            ) : null}
        </div>
    )
}
