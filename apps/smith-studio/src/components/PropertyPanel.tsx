import { useEffect, useState } from "react";
import type { Node } from "@xyflow/react";
import { stepData } from "../lib/robot";
import type { StepParams } from "../types";

interface PropertyPanelProps {
  node: Node | null;
  onUpdate: (id: string, action: string, params: StepParams) => void;
  onDelete: (id: string) => void;
  onMove: (id: string, delta: -1 | 1) => void;
}

/** Правая панель: редактирование выбранного шага (action + params JSON). */
export function PropertyPanel({ node, onUpdate, onDelete, onMove }: PropertyPanelProps) {
  const [paramsText, setParamsText] = useState("{}");
  const [paramsError, setParamsError] = useState<string | null>(null);

  const data = node ? stepData(node) : null;

  // Пересоздаём локальный JSON-буфер при смене выделения.
  useEffect(() => {
    if (data) {
      setParamsText(JSON.stringify(data.params, null, 2));
      setParamsError(null);
    }
  }, [node?.id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!node || !data) {
    return (
      <aside className="w-80 border-l border-slate-200 bg-white p-4 text-sm text-slate-400">
        Выберите шаг на холсте, чтобы изменить его параметры.
      </aside>
    );
  }

  const handleParamsChange = (text: string) => {
    setParamsText(text);
    try {
      const parsed: unknown = JSON.parse(text);
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        throw new Error("Параметры должны быть JSON-объектом");
      }
      setParamsError(null);
      onUpdate(node.id, data.action, parsed as StepParams);
    } catch (error) {
      setParamsError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleActionChange = (action: string) => {
    onUpdate(node.id, action, data.params);
  };

  return (
    <aside className="w-80 overflow-y-auto border-l border-slate-200 bg-white p-4">
      <h2 className="mb-3 text-xs font-semibold uppercase tracking-wide text-slate-500">
        Шаг {data.index + 1}
      </h2>

      <label className="mb-1 block text-sm font-medium text-slate-700">Инструмент</label>
      <input
        value={data.action}
        onChange={(e) => handleActionChange(e.target.value)}
        className="mb-4 w-full rounded-md border border-slate-300 px-2 py-1.5 font-mono text-sm"
      />

      <label className="mb-1 block text-sm font-medium text-slate-700">Параметры (JSON)</label>
      <textarea
        value={paramsText}
        onChange={(e) => handleParamsChange(e.target.value)}
        rows={12}
        spellCheck={false}
        className={`w-full rounded-md border px-2 py-1.5 font-mono text-xs ${
          paramsError ? "border-red-400" : "border-slate-300"
        }`}
      />
      {paramsError && <p className="mt-1 text-xs text-red-600">{paramsError}</p>}

      <div className="mt-4 flex gap-2">
        <button
          type="button"
          onClick={() => onMove(node.id, -1)}
          className="flex-1 rounded-md border border-slate-300 px-2 py-1 text-sm hover:bg-slate-50"
        >
          ↑ Вверх
        </button>
        <button
          type="button"
          onClick={() => onMove(node.id, 1)}
          className="flex-1 rounded-md border border-slate-300 px-2 py-1 text-sm hover:bg-slate-50"
        >
          ↓ Вниз
        </button>
      </div>
      <button
        type="button"
        onClick={() => onDelete(node.id)}
        className="mt-2 w-full rounded-md border border-red-300 bg-red-50 px-2 py-1 text-sm text-red-700 hover:bg-red-100"
      >
        Удалить шаг
      </button>
    </aside>
  );
}
