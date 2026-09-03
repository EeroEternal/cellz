# Admin UI kit

Copied from xrouter `admin/src/components/ui` + tokens in `src/index.css`.

This is the **vocabulary** `docs/design.md` binds to. Agents must implement product UI here (React), never as Greenfield HTML.

Sidebar entries live in `src/lib/nav.ts`. Add a page = add a nav item + `pages/*.tsx` + a route in `App.tsx`. Do not invent a second menu.

Example reader jobs (replace mock data):
- Dashboard: `StatCard` + `InsightPanel` (title only) + `MetricTile`
- List: `EntityListToolbar` + table, quiet selection `bg-primary/10`
- Catalog: `TwoPanelLayout` + `DetailPanel`
- Workbench: history + working pane
- Settings: `SectionCard` on this page only; no casual subtitle

Kit copy list: [`KIT.md`](KIT.md). Upgrade another repo with `scripts/sync-admin-kit.sh`.

```bash
cd admin && npm install && npm run dev
npm run lint
bash ../scripts/check_ui_stack.sh && bash ../scripts/check_admin_nav.sh
```
