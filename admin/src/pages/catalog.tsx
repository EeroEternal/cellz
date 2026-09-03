import { useState } from "react"
import { DetailPanel } from "@/components/layout/detail-panel"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { TwoPanelLayout } from "@/components/layout/two-panel-layout"
import { Card } from "@/components/ui/card"

const ITEMS = [
  { id: "mod-1", name: "Alpha" },
  { id: "mod-2", name: "Beta" },
  { id: "mod-3", name: "Gamma" },
]

export default function CatalogPage() {
  const [selectedId, setSelectedId] = useState(ITEMS[0].id)
  const selected = ITEMS.find((item) => item.id === selectedId) ?? ITEMS[0]

  return (
    <PageShell>
      <PageContainer>
        <PageHeader title="Catalog" />
        <TwoPanelLayout
          workspace
          left={
            <Card className="min-h-0 flex-1 gap-0 overflow-hidden p-0">
              <ul className="h-full overflow-y-auto p-2">
                {ITEMS.map((item) => (
                  <li key={item.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(item.id)}
                      className={`flex w-full rounded-md px-3 py-2 text-left text-sm leading-5 ${
                        item.id === selectedId
                          ? "bg-primary/10 font-medium text-primary"
                          : "hover:bg-muted"
                      }`}
                    >
                      {item.name}
                    </button>
                  </li>
                ))}
              </ul>
            </Card>
          }
          right={
            <DetailPanel>
              <h2 className="text-section-title">{selected.name}</h2>
              <p className="text-meta-sm mt-2 text-muted-foreground">{selected.id}</p>
            </DetailPanel>
          }
        />
      </PageContainer>
    </PageShell>
  )
}
