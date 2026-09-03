import * as React from "react"
import { Link, useLocation } from "react-router-dom"
import { ChevronDown, ChevronUp } from "lucide-react"

import {
  Sidebar,
  SidebarContent,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  useSidebar,
} from "@/components/ui/sidebar"
import { APP_TITLE, NAV_SECTIONS, type NavItem, type NavSection } from "@/lib/nav"
import { cn } from "@/lib/utils"

const SECTION_PREVIEW_COUNT = 3
const SECTION_EXPAND_STORAGE_KEY = "admin.nav-section-expanded.v1"

function splitVisibleItems(items: NavItem[], expanded: boolean): NavItem[] {
  if (expanded) return items
  return items.slice(0, SECTION_PREVIEW_COUNT)
}

function loadExpandedSections(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(SECTION_EXPAND_STORAGE_KEY)
    if (!raw) return {}
    const parsed = JSON.parse(raw) as Record<string, boolean>
    return parsed && typeof parsed === "object" ? parsed : {}
  } catch {
    return {}
  }
}

function NavSectionGroup({
  section,
  pathname,
}: {
  section: NavSection
  pathname: string
}) {
  const { state } = useSidebar()
  const [expandedMap, setExpandedMap] = React.useState<Record<string, boolean>>(loadExpandedSections)
  const collapsedItems = splitVisibleItems(section.items, false)
  const canCollapse =
    section.collapsible === true && collapsedItems.length < section.items.length
  const hiddenContainsActive =
    canCollapse &&
    section.items.some(
      (item) => pathname === item.href && !collapsedItems.some((visible) => visible.href === item.href)
    )
  const expanded = !canCollapse || state !== "expanded" || expandedMap[section.id] === true || hiddenContainsActive
  const visibleItems = splitVisibleItems(section.items, expanded)

  const toggleExpanded = () => {
    setExpandedMap((current) => {
      const next = { ...current, [section.id]: !expanded }
      localStorage.setItem(SECTION_EXPAND_STORAGE_KEY, JSON.stringify(next))
      return next
    })
  }

  return (
    <SidebarGroup>
      <SidebarGroupLabel className="text-sidebar-foreground/85">{section.title}</SidebarGroupLabel>
      <SidebarMenu>
        {visibleItems.map((item) => {
          const isActive = pathname === item.href
          return (
            <SidebarMenuItem key={item.href}>
              <SidebarMenuButton asChild tooltip={item.name} isActive={isActive}>
                <Link to={item.href} className={cn("flex items-center gap-3")}>
                  <item.icon className="size-4" />
                  <span>{item.name}</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          )
        })}
        {canCollapse && state === "expanded" && !hiddenContainsActive ? (
          <SidebarMenuItem>
            <SidebarMenuButton
              type="button"
              tooltip={expanded ? "Show less" : "Show more"}
              onClick={toggleExpanded}
              className="text-sidebar-foreground/80"
            >
              {expanded ? <ChevronUp className="size-4" /> : <ChevronDown className="size-4" />}
              <span>{expanded ? "Show less" : "Show more"}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        ) : null}
      </SidebarMenu>
    </SidebarGroup>
  )
}

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const location = useLocation()

  return (
    <Sidebar collapsible="icon" {...props}>
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton size="lg" asChild>
              <Link to="/">
                <div className="flex size-8 items-center justify-center rounded-md bg-primary text-primary-foreground text-sm font-semibold">
                  A
                </div>
                <div className="grid min-w-0 flex-1 text-left text-sm leading-tight">
                  <span className="break-words font-semibold text-base leading-5">{APP_TITLE}</span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>
      <SidebarContent>
        {NAV_SECTIONS.map((section) => (
          <NavSectionGroup
            key={section.id}
            section={section}
            pathname={location.pathname}
          />
        ))}
      </SidebarContent>
      <SidebarRail />
    </Sidebar>
  )
}
