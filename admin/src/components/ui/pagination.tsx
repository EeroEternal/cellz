import { useState } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Select } from "@/components/ui/select"
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from "lucide-react"
import { t } from "@/lib/i18n"

interface PaginationProps {
  currentPage: number
  pageSize: number
  totalCount: number
  onPageChange: (page: number) => void
  onPageSizeChange?: (size: number) => void
  disabled?: boolean
}

export function Pagination({
  currentPage,
  pageSize,
  totalCount,
  onPageChange,
  onPageSizeChange,
  disabled = false,
}: PaginationProps) {
  const [jumpPage, setJumpPage] = useState("")
  const totalPages = Math.ceil(totalCount / pageSize)
  const startItem = totalCount === 0 ? 0 : (currentPage - 1) * pageSize + 1
  const endItem = Math.min(currentPage * pageSize, totalCount)
  const summary = t("common.paginationSummary")
    .replace("{start}", String(startItem))
    .replace("{end}", String(endItem))
    .replace("{total}", String(totalCount))

  const getPageNumbers = () => {
    const pages: (number | "ellipsis")[] = []
    if (totalPages <= 7) {
      for (let i = 1; i <= totalPages; i++) pages.push(i)
    } else {
      pages.push(1)
      if (currentPage > 3) pages.push("ellipsis")
      
      const start = Math.max(2, currentPage - 1)
      const end = Math.min(totalPages - 1, currentPage + 1)
      for (let i = start; i <= end; i++) pages.push(i)
      
      if (currentPage < totalPages - 2) pages.push("ellipsis")
      pages.push(totalPages)
    }
    return pages
  }

  const handleJump = () => {
    const page = parseInt(jumpPage)
    if (page >= 1 && page <= totalPages) {
      onPageChange(page)
      setJumpPage("")
    }
  }

  return (
    <div className="flex flex-col gap-3 border-t px-4 py-3 xl:flex-row xl:items-center xl:justify-between" onClick={(e) => e.stopPropagation()}>
      <div className="flex flex-wrap items-center gap-4">
        <span className="text-sm text-muted-foreground">
          {summary}
        </span>
        {onPageSizeChange && (
          <div className="flex items-center gap-2">
            <span className="text-sm text-muted-foreground">{t("common.perPage")}</span>
            <Select
              value={String(pageSize)}
              onChange={(value) => onPageSizeChange(Number(value))}
              options={[10, 20, 50, 100].map((size) => ({ value: String(size), label: String(size) }))}
              disabled={disabled}
              className="w-20"
              triggerClassName="h-8"
            />
          </div>
        )}
      </div>

      <div className="flex flex-wrap items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0"
          onClick={() => onPageChange(1)}
          disabled={currentPage === 1 || disabled}
          aria-label={t("common.firstPage")}
          title={t("common.firstPage")}
        >
          <ChevronsLeft className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0"
          onClick={() => onPageChange(currentPage - 1)}
          disabled={currentPage === 1 || disabled}
          aria-label={t("common.previousPage")}
          title={t("common.previousPage")}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>

        {getPageNumbers().map((page, index) =>
          page === "ellipsis" ? (
            <button
              key={`ellipsis-${index}`}
              className="h-8 w-8 flex items-center justify-center text-sm text-muted-foreground hover:text-foreground"
              onClick={() => {
                const pages = getPageNumbers()
                const prevPage = pages[index - 1]
                if (typeof prevPage === "number") {
                  onPageChange(Math.min(prevPage + 5, totalPages))
                }
              }}
              disabled={disabled}
              aria-label={t("common.morePages")}
            >
              ...
            </button>
          ) : (
            <Button
              key={page}
              variant={page === currentPage ? "default" : "ghost"}
              size="sm"
              className={`h-8 w-8 p-0 text-sm ${
                page === currentPage
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => onPageChange(page)}
              disabled={disabled}
              aria-current={page === currentPage ? "page" : undefined}
            >
              {page}
            </Button>
          )
        )}

        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0"
          onClick={() => onPageChange(currentPage + 1)}
          disabled={currentPage === totalPages || disabled}
          aria-label={t("common.nextPage")}
          title={t("common.nextPage")}
        >
          <ChevronRight className="h-4 w-4" />
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="h-8 w-8 p-0"
          onClick={() => onPageChange(totalPages)}
          disabled={currentPage === totalPages || disabled}
          aria-label={t("common.lastPage")}
          title={t("common.lastPage")}
        >
          <ChevronsRight className="h-4 w-4" />
        </Button>

        <div className="flex items-center gap-1 ml-2">
          <Input
            type="number"
            min={1}
            max={totalPages}
            placeholder={t("common.jumpPlaceholder")}
            aria-label={t("common.jumpPlaceholder")}
            className="h-8 w-16 rounded border border-input bg-background px-2 text-sm text-center"
            value={jumpPage}
            onChange={(e) => setJumpPage(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleJump()
            }}
            disabled={disabled}
          />
          <Button
            variant="ghost"
            size="sm"
            className="h-8 px-2 text-sm"
            onClick={handleJump}
            disabled={disabled}
          >
            {t("common.jumpToPage")}
          </Button>
        </div>
      </div>
    </div>
  )
}
