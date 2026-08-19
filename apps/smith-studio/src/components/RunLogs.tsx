import type { ExecutionReportView, JobView } from "../types";
import { Badge } from "./ui/badge";

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


/** Нижняя панель: статус текущего запуска, пошаговый отчёт и история. */
export function RunLogs({ jobStatus, report, history }: RunLogsProps) {
  const statusLabel = jobStatus ? (STATUS_LABEL[jobStatus] ?? jobStatus) : null;

  const badgeVariant = (status: string): "default" | "secondary" | "destructive" | "outline" => {
    switch (status) {
      case "succeeded": return "default";
      case "failed": return "destructive";
      case "running": return "secondary";
      default: return "outline";
    }
  };

  return (
    <div className="flex max-h-56 flex-col overflow-hidden border-t border-border bg-card">
      <div className="flex min-h-0 flex-1">
        {/* Текущий запуск */}
        <section className="flex w-1/2 flex-col border-r border-border p-3">
          <div className="mb-2 flex items-center gap-2">
            <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
              Запуск
            </h2>
            {statusLabel && (
              <Badge variant={badgeVariant(jobStatus ?? "")}>
                {statusLabel}
              </Badge>
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
                  <span className="mr-2 text-muted-foreground">{i + 1}.</span>
                  <span className="font-medium">{step.action}</span>
                  {step.ok && step.output !== undefined && (
                    <span className="ml-2 text-muted-foreground">{JSON.stringify(step.output)}</span>
                  )}
                  {!step.ok && step.error && <span className="ml-2">{step.error}</span>}
                </li>
              ))}
            </ol>
          ) : (
            <p className="text-sm text-muted-foreground">
              {jobStatus ? "Робот выполняется…" : "Запустите робота, чтобы увидеть логи."}
            </p>
          )}
        </section>

        {/* История */}
        <section className="flex w-1/2 flex-col p-3">
          <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            История запусков
          </h2>
          {history.length === 0 ? (
            <p className="text-sm text-muted-foreground">Запусков пока нет.</p>
          ) : (
            <ul className="min-h-0 flex-1 space-y-1 overflow-y-auto font-mono text-xs">
              {[...history].reverse().map((job) => (
                <li
                  key={job.id}
                  className="flex items-center gap-2 rounded px-2 py-1 bg-secondary/50"
                >
                  <span className="text-muted-foreground">#{job.id}</span>
                  <span className="truncate text-foreground">{job.robot_name}</span>
                  <Badge variant={badgeVariant(job.status)} className="ml-auto">
                    {STATUS_LABEL[job.status] ?? job.status}
                  </Badge>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
