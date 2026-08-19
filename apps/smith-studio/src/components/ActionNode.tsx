import { Handle, Position, type NodeProps } from "@xyflow/react";

interface ActionNodeExtraData {
  index: number;
  action: string;
  params: Record<string, unknown>;
  outputs: Record<string, string>;
  /** Точка останова на этом шаге. */
  breakpoint?: boolean;
  /** Текущий выполняющийся шаг (для подсветки amber). */
  currentStep?: number;
  /** Режим отладки активен. */
  debugMode?: boolean;
}

/** Узел шага робота: номер, имя инструмента и параметры. */
export function ActionNode({ data, selected }: NodeProps) {
  const d = data as unknown as ActionNodeExtraData;
  const paramCount = Object.keys(d.params).length;
  const isActive = d.debugMode && d.currentStep === d.index;

  return (
    <div
      className={`w-36 rounded border bg-white px-1.5 py-1 shadow-sm ${
        d.breakpoint
          ? "border-red-500 ring-2 ring-red-200"
          : isActive
            ? "border-amber-500 ring-2 ring-amber-200"
            : selected
              ? "border-blue-500 ring-2 ring-blue-200"
              : "border-slate-300"
      }`}
    >
      <Handle type="target" position={Position.Left} />
      <div className="flex items-center gap-1">
        <span className="rounded bg-slate-100 px-0.5 py-px font-mono text-[9px] text-slate-500">
          {d.index + 1}
        </span>
        <span className="truncate font-mono text-[11px] font-medium text-slate-800">{d.action}</span>
        {d.breakpoint && (
          <span className="ml-auto block h-2 w-2 rounded-full bg-red-500" title="Точка останова (F9)" />
        )}
      </div>
      {paramCount > 0 && (
        <pre className="mt-px max-h-10 overflow-hidden text-[8px] leading-tight text-slate-500">
          {JSON.stringify(d.params, null, 1)}
        </pre>
      )}
      {Object.keys(d.outputs).length > 0 && (
        <div className="mt-px space-y-px">
          {Object.entries(d.outputs).map(([field, varName]) => (
            <div
              key={field}
              className="truncate rounded bg-blue-50 px-1 py-px font-mono text-[8px] text-blue-700"
            >
              {field} → {varName}
            </div>
          ))}
        </div>
      )}
      <Handle type="source" position={Position.Right} />
    </div>
  );
}
