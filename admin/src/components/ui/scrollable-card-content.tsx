import { useEffect, useRef, useState, type ReactNode } from "react"
import { t } from "@/lib/i18n"
import { cn } from "@/lib/utils"

interface ScrollableCardContentProps {
  children: ReactNode
  className?: string
  viewportClassName?: string
  contentClassName?: string
}

export function ScrollableCardContent({
  children,
  className,
  viewportClassName,
  contentClassName,
}: ScrollableCardContentProps) {
  const viewportRef = useRef<HTMLDivElement | null>(null)
  const [isScrollable, setIsScrollable] = useState(false)
  const [isAtTop, setIsAtTop] = useState(true)
  const [isAtBottom, setIsAtBottom] = useState(true)

  useEffect(() => {
    const element = viewportRef.current
    if (!element) {
      return
    }

    const updateState = () => {
      const scrollable = element.scrollHeight - element.clientHeight > 4
      const atTop = element.scrollTop <= 4
      const atBottom = element.scrollTop + element.clientHeight >= element.scrollHeight - 4
      setIsScrollable(scrollable)
      setIsAtTop(atTop)
      setIsAtBottom(atBottom)
    }

    updateState()

    const resizeObserver = new ResizeObserver(() => {
      updateState()
    })

    resizeObserver.observe(element)
    Array.from(element.children).forEach((child) => resizeObserver.observe(child))

    element.addEventListener("scroll", updateState, { passive: true })
    window.addEventListener("resize", updateState)

    return () => {
      resizeObserver.disconnect()
      element.removeEventListener("scroll", updateState)
      window.removeEventListener("resize", updateState)
    }
  }, [children])

  return (
    <div className={cn("relative min-h-0 flex-1", className)}>
      {isScrollable ? (
        <div className="pointer-events-none absolute right-2 top-2 z-10 rounded-md bg-background/72 px-2 py-1 text-[9px] font-medium tracking-[0.06em] text-muted-foreground/80 shadow-[0_1px_2px_rgba(0,0,0,0.04)] ring-1 ring-border/35 backdrop-blur-sm">
          {t("common.scrollableHint", "Scrollable")}
        </div>
      ) : null}

      {isScrollable && !isAtTop ? (
        <div className="pointer-events-none absolute inset-x-0 top-0 z-[1] h-8 bg-gradient-to-b from-background via-background/80 to-transparent" />
      ) : null}

      <div
        ref={viewportRef}
        className={cn(
          "scrollbar-hover min-h-0 h-full overflow-auto pr-1",
          isScrollable ? "scrollable-card-content" : "scrollbar-hide",
          isScrollable ? "pt-8" : "",
          viewportClassName
        )}
      >
        <div className={cn("min-h-full", contentClassName)}>{children}</div>
      </div>

      {isScrollable && !isAtBottom ? (
        <div className="pointer-events-none absolute inset-x-0 bottom-0 z-[1] h-10 bg-gradient-to-t from-background via-background/85 to-transparent" />
      ) : null}
    </div>
  )
}