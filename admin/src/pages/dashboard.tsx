import { Activity, Server } from "lucide-react"
import {
  analyticsGridClassName,
  dashboardMonitoringCardClassName,
  dashboardMonitoringHeaderClassName,
} from "@/common/card-height-presets"
import { InsightList } from "@/common/insight-list"
import { InsightListItem } from "@/common/insight-list-item"
import { InsightPanel } from "@/common/insight-panel"
import { MetricTile } from "@/common/metric-tile"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { StatCard } from "@/components/ui/stat-card"

export default function DashboardPage() {
  return (
    <PageShell className="overflow-y-auto">
      <PageContainer>
        <PageHeader title="Dashboard" />
        <div className="space-y-6 pb-6">
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-4">
            <StatCard title="Requests today" value="2,560" icon={Activity} />
            <StatCard title="Online nodes" value="3 / 3" icon={Server} />
          </div>

          <div className={analyticsGridClassName}>
            <InsightPanel title="Top models">
              <InsightList>
                <InsightListItem>
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-sm font-medium">alpha</span>
                    <span className="text-meta-sm text-muted-foreground">1,240</span>
                  </div>
                </InsightListItem>
                <InsightListItem>
                  <div className="flex items-center justify-between gap-2">
                    <span className="text-sm font-medium">beta</span>
                    <span className="text-meta-sm text-muted-foreground">860</span>
                  </div>
                </InsightListItem>
              </InsightList>
            </InsightPanel>
            <div className="min-h-0 xl:col-span-8">
              <Card className={dashboardMonitoringCardClassName}>
                <CardHeader className={dashboardMonitoringHeaderClassName}>
                  <CardTitle>Traffic</CardTitle>
                </CardHeader>
                <CardContent className="grid min-h-0 flex-1 grid-cols-2 content-start gap-2 px-5 pb-5">
                  <MetricTile label="Success rate" value="99.5%" />
                  <MetricTile label="Avg latency" value="42ms" />
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </PageContainer>
    </PageShell>
  )
}
