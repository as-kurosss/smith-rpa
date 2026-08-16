import { TOOL_CATALOG } from "../lib/toolCatalog";
import type { ToolDef } from "../types";

interface NodePaletteProps {
  onAdd: (tool: ToolDef) => void;
}

/** Левая панель: библиотека узлов (инструменты). Клик добавляет шаг на холст. */
export function NodePalette({ onAdd }: NodePaletteProps) {
  return (
    <aside className="w-64 overflow-y-auto border-r border-slate-200 bg-white p-3">
      <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-slate-500">
        Инструменты
      </h2>
      <ul className="space-y-1.5">
        {TOOL_CATALOG.map((tool) => (
          <li key={tool.name}>
            <button
              type="button"
              onClick={() => onAdd(tool)}
              className="w-full rounded-md border border-slate-200 bg-slate-50 px-3 py-2 text-left transition hover:border-blue-400 hover:bg-blue-50"
              title={tool.description}
            >
              <span className="block font-mono text-sm font-medium text-slate-800">
                {tool.label}
              </span>
              <span className="block truncate font-mono text-xs text-slate-400">
                {tool.name}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </aside>
  );
}
