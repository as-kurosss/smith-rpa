import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { ActionNodeData } from "../types";

/** Узел шага робота: номер, имя инструмента и параметры. */
export function ActionNode({ data, selected }: NodeProps) {
  const d = data as unknown as ActionNodeData;
  const paramCount = Object.keys(d.params).length;

  return (
    <div
      className={`w-36 rounded border bg-white px-1.5 py-1 shadow-sm ${
        selected ? "border-blue-500 ring-2 ring-blue-200" : "border-slate-300"
      }`}
    >
      <Handle type="target" position={Position.Left} />
      <div className="flex items-center gap-1">
        <span className="rounded bg-slate-100 px-0.5 py-px font-mono text-[9px] text-slate-500">
          {d.index + 1}
        </span>
        <span className="truncate font-mono text-[11px] font-medium text-slate-800">{d.action}</span>
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
