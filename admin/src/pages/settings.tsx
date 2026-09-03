import { SectionCard } from "@/common/section-card"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"

export default function SettingsPage() {
  return (
    <PageShell className="overflow-y-auto">
      <PageContainer>
        <PageHeader title="Settings" />
        <SectionCard title="General">
          <p className="text-body-md text-muted-foreground">
            Global configuration lives on this page only.
          </p>
        </SectionCard>
      </PageContainer>
    </PageShell>
  )
}
