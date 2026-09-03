import { useMemo, useState } from "react"
import { PageContainer } from "@/components/layout/page-container"
import { PageHeader } from "@/components/layout/page-header"
import { PageShell } from "@/components/layout/page-shell"
import { EntityListToolbar } from "@/components/entity-list/EntityListToolbar"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Select } from "@/components/ui/select"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"

const ROWS = [
  { id: "svc-1", name: "chat-prod", status: "active" },
  { id: "svc-2", name: "embed-prod", status: "disabled" },
  { id: "svc-3", name: "image-lab", status: "active" },
]

export default function ListPage() {
  const [query, setQuery] = useState("")
  const [status, setStatus] = useState("all")
  const [selected, setSelected] = useState<string | null>(null)

  const rows = useMemo(() => {
    return ROWS.filter((row) => {
      const matchesQuery =
        !query || row.name.includes(query) || row.id.includes(query)
      const matchesStatus = status === "all" || row.status === status
      return matchesQuery && matchesStatus
    })
  }, [query, status])

  return (
    <PageShell>
      <PageContainer>
        <PageHeader
          title="List"
          action={<Button>Create</Button>}
        />
        <Card className="gap-0 p-4 sm:p-6">
          <EntityListToolbar
            searchValue={query}
            onSearchChange={setQuery}
            searchPlaceholder="Search name, id"
            filters={
              <Select
                value={status}
                onChange={setStatus}
                className="w-40"
                options={[
                  { value: "all", label: "All statuses" },
                  { value: "active", label: "Active" },
                  { value: "disabled", label: "Disabled" },
                ]}
              />
            }
            resultCount={`${rows.length} items`}
          />
          <Table className="table-fixed">
            <TableHeader className="sticky top-0 bg-card">
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>ID</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((row) => (
                <TableRow
                  key={row.id}
                  className={selected === row.id ? "bg-primary/10 font-medium" : "hover:bg-muted/50"}
                  onClick={() => setSelected(row.id)}
                >
                  <TableCell>{row.name}</TableCell>
                  <TableCell>{row.status}</TableCell>
                  <TableCell className="font-mono text-xs">{row.id}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Card>
      </PageContainer>
    </PageShell>
  )
}
