import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom"
import { DashboardLayout } from "@/components/layout/dashboard-layout"
import CatalogPage from "@/pages/catalog"
import DashboardPage from "@/pages/dashboard"
import ListPage from "@/pages/list"
import SettingsPage from "@/pages/settings"
import WorkbenchPage from "@/pages/workbench"

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<DashboardLayout />}>
          <Route index element={<DashboardPage />} />
          <Route path="list" element={<ListPage />} />
          <Route path="catalog" element={<CatalogPage />} />
          <Route path="workbench" element={<WorkbenchPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  )
}
