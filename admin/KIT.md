# Admin kit copy surface

This directory is a **source kit**, not an npm library. New products copy these files into their tree so `docs/design.md` can bind.

| Copy (required) | Optional | Do not treat as spec |
| --- | --- | --- |
| `src/components/ui/*` | `src/pages/*` (examples) | Product dialogs and domain cards |
| `src/components/layout/*` | `src/lib/nav.ts` | Locale dictionaries |
| `src/common/*` | `src/App.tsx` | xrouter business pages |
| `src/index.css`, `tailwind.config.js` | | |
| `src/lib/utils.ts`, `src/lib/i18n.ts` (stub) | | |
| `docs/design.md` + `docs/design/` | | |
| `.agents/skills/admin-ui-change` | | |

Upgrade: `./scripts/sync-admin-kit.sh <product-root>` (skips `pages/` and `nav.ts`).
