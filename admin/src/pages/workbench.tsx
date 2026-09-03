import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { Card } from "@/components/ui/card"

export default function WorkbenchPage() {
  return (
    <PageShell className="overflow-hidden">
      <PageContainer>
        <PageHeader title="Workbench" />
        <div className="flex min-h-0 flex-1 gap-4">
          <Card className="w-64 shrink-0 gap-0 overflow-y-auto p-3">
            <p className="text-label-sm text-muted-foreground">History</p>
          </Card>
          <Card className="min-h-0 min-w-0 flex-1 gap-0 p-4">
            <p className="text-body-md text-muted-foreground">Working pane</p>
          </Card>
        </div>
      </PageContainer>
    </PageShell>
  )
}
