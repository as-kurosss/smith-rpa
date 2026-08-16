import type { ExecutionReportView, JobView } from "../types";

interface RunLogsProps {
  jobStatus: string | null;
  report: ExecutionReportView | null;
  history: JobView[];
}

const STATUS_LABEL: Record<string, string> = {
  queued: "В очереди",
  running: "Выполняется",
  succeeded: "Успех",
  failed: "Ошибка",
  cancelled: "Отменён",
};

const STATUS_COLOR: Record<string, string> = {
  queued: "bg-slate-200 text-slate-700",
  running: "bg-amber-100 text-amber-800",
  succeeded: "bg-emerald-100 text-emerald-800",
  failed: "bg-red-100 text-red-800",
  cancelled: "bg-slate-200 text-slate-600",
};

/** Нижняя панель: статус текущего запуска, пошаговый отчёт и история. */
export function RunLogs({ jobStatus, report, history }: RunLogsProps) {
  const statusLabel = jobStatus ? (STATUS_LABEL[jobStatus] ?? jobStatus) : null;
  const statusColor = jobStatus ? (STATUS_COLOR[jobStatus] ?? "bg-slate-200 text-slate-700") : null;

  return (
    <div className="flex max-h-56 flex-col overflow-hidden border-t border-slate-200 bg-white">
      <div className="flex min-h-0 flex-1">
        {/* Текущий запуск */}
        <section className="flex w-1/2 flex-col border-r border-slate-100 p-3">
          <div className="mb-2 flex items-center gap-2">
            <h2 className="text-xs font-semibold uppercase tracking-wide text-slate-500">
              Запуск
            </h2>
            {statusLabel && (
              <span className={`rounded-full px-2 py-0.5 text-xs font-medium ${statusColor}`}>
                {statusLabel}
              </span>
            )}
          </div>
          {report ? (
            <ol className="min-h-0 flex-1 space-y-1 overflow-y-auto font-mono text-xs">
              {report.steps.map((step, i) => (
                <li
                  key={i}
                  className={`rounded px-2 py-1 ${
                    step.ok ? "bg-emerald-50 text-emerald-900" : "bg-red-50 text-red-900"
                  }`}
                >
                  <span className="mr-2 text-slate-400">{i + 1}.</span>
                  <span className="font-medium">{step.action}</span>
                  {step.ok && step.output !== undefined && (
                    <span className="ml-2 text-slate-500">{JSON.stringify(step.output)}</span>
                  )}
                  {!step.ok && step.error && <span className="ml-2">{step.error}</span>}
                </li>
              ))}
            </ol>
          ) : (
            <p className="text-sm text-slate-400">
              {jobStatus ? "Робот выполняется…" : "Запустите робота, чтобы увидеть логи."}
            </p>
          )}
        </section>

        {/* История */}
        <section className="flex w-1/2 flex-col p-3">
          <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-500">
            История запусков
          </h2>
          {history.length === 0 ? (
            <p className="text-sm text-slate-400">Запусков пока нет.</p>
          ) : (
            <ul className="min-h-0 flex-1 space-y-1 overflow-y-auto font-mono text-xs">
              {[...history].reverse().map((job) => (
                <li
                  key={job.id}
                  className="flex items-center gap-2 rounded px-2 py-1 bg-slate-50"
                >
                  <span className="text-slate-400">#{job.id}</span>
                  <span className="truncate text-slate-700">{job.robot_name}</span>
                  <span
                    className={`ml-auto rounded-full px-2 py-0.5 text-xs font-medium ${
                      STATUS_COLOR[job.status] ?? "bg-slate-200 text-slate-700"
                    }`}
                  >
                    {STATUS_LABEL[job.status] ?? job.status}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
