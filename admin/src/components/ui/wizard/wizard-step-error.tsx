export function WizardStepError({ message }: { message: string }) {
  return (
    <div className="rounded-md border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive">
      {message}
    </div>
  )
}
