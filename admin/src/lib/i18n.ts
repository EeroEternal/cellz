/** Kit stub: prefer `t("key", "Visible label")` until product locales exist. */
export function t(key: string, fallback?: string): string {
  if (fallback && fallback !== "zh" && fallback !== "en") return fallback
  return key
}

export function useI18n() {
  return { language: "en" as const, setLanguage: () => {} }
}
