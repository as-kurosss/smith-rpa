import { Handle, Position, type NodeProps } from "@xyflow/react";

interface ActionNodeExtraData {
  /** Индекс шага (0-based). */
  index: number;
  action: string;
  params: Record<string, unknown>;
  outputs: Record<string, string>;
  /** Установлена ли точка останова на этом шаге. */
  breakpoint?: boolean;
  /** Текущий выполняющийся шаг (0-based). Совпадает с `index` если пауза перед этим шагом. */
  currentStep?: number;
  /** Режим отладки активен. */
  debugMode?: boolean;
  /** Обработчик клика по gutter (toggle breakpoint). */
  onToggleBreakpoint?: (stepIndex: number) => void;
}

/** Узел шага робота: gutter (breakpoint), номер, имя инструмента и параметры. */
export function ActionNode({ data, selected }: NodeProps) {
  const d = data as unknown as ActionNodeExtraData;
  const paramCount = Object.keys(d.params).length;
  const isActive = d.debugMode && d.currentStep === d.index;

  return (
    <div
      className={`relative w-36 rounded border bg-white shadow-sm ${
        isActive
          ? "border-amber-500 ring-2 ring-amber-200"
          : selected
            ? "border-blue-500 ring-2 ring-blue-200"
            : "border-slate-300"
      }`}
    >
      <Handle type="target" position={Position.Left} />
      {/* Gutter: клик ставит / снимает breakpoint */}
      {d.debugMode && (
        <button
          type="button"
          className="absolute inset-y-0 left-0 flex w-4 cursor-pointer items-center justify-center border-r border-transparent hover:bg-slate-100"
          onClick={(e) => {
            e.stopPropagation();
            d.onToggleBreakpoint?.(d.index);
          }}
          title="Точка останова"
        >
          {d.breakpoint && (
            <span className="block h-2.5 w-2.5 rounded-full bg-red-500" />
          )}
        </button>
      )}
      <div className={`flex items-center gap-1 ${d.debugMode ? "pl-5" : "px-1.5"} py-1`}>
        <span className="rounded bg-slate-100 px-0.5 py-px font-mono text-[9px] text-slate-500">
          {d.index + 1}
        </span>
        <span className="truncate font-mono text-[11px] font-medium text-slate-800">{d.action}</span>
      </div>
      {paramCount > 0 && (
        <pre className="mt-px max-h-10 overflow-hidden px-1.5 text-[8px] leading-tight text-slate-500">
          {JSON.stringify(d.params, null, 1)}
        </pre>
      )}
      {Object.keys(d.outputs).length > 0 && (
        <div className="mt-px space-y-px px-1.5">
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
