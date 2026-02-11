export default function App() {
  return (
    <main className="min-h-screen bg-slate-50 text-slate-900">
      <section className="mx-auto flex min-h-screen w-full max-w-3xl flex-col items-center justify-center gap-4 px-6 text-center">
        <p className="rounded-full bg-brand-100 px-3 py-1 text-xs font-semibold uppercase tracking-[0.14em] text-brand-700">
          Step-01 Baseline
        </p>
        <h1 className="text-4xl font-bold tracking-tight">EngineQA</h1>
        <p className="text-base text-slate-600">
          Frontend scaffold is ready. Next steps will wire `/api/query`, `/api/status`, and `/api/feedback`.
        </p>
      </section>
    </main>
  );
}
