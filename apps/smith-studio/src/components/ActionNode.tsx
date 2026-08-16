import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { ActionNodeData } from "../types";

/** Узел шага робота: номер, имя инструмента и параметры. */
export function ActionNode({ data, selected }: NodeProps) {
  const d = data as unknown as ActionNodeData;
  const paramCount = Object.keys(d.params).length;

  return (
    <div
      className={`w-60 rounded-lg border bg-white px-3 py-2 shadow-sm ${
        selected ? "border-blue-500 ring-2 ring-blue-200" : "border-slate-300"
      }`}
    >
      <Handle type="target" position={Position.Left} />
      <div className="flex items-center gap-2">
        <span className="rounded bg-slate-100 px-1.5 py-0.5 font-mono text-xs text-slate-500">
          {d.index + 1}
        </span>
        <span className="truncate font-mono text-sm font-medium text-slate-800">{d.action}</span>
      </div>
      {paramCount > 0 && (
        <pre className="mt-1 max-h-16 overflow-hidden text-[10px] leading-tight text-slate-500">
          {JSON.stringify(d.params, null, 1)}
        </pre>
      )}
      <Handle type="source" position={Position.Right} />
    </div>
  );
}
