import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";

/** Одна переменная контекста. */
interface ContextVar {
  name: string;
  type_name: string;
  value: string;
}

interface DebugConsoleProps {
  /** ID текущего выполняющегося или завершённого джоба. */
  currentJobId: number | null;
  /** Добавить шаг с element_key = variable name. */
  onAddClick?: (elementKey: string) => void;
}

/** Нижняя панель: таблица переменных контекста в реальном времени. */
export function DebugConsole({ currentJobId, onAddClick }: DebugConsoleProps) {
  const [vars, setVars] = useState<ContextVar[]>([]);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchVars = useCallback(async () => {
    if (currentJobId === null) {
      setVars([]);
      return;
    }
    try {
      const snapshot = await invoke<Record<string, { type_name: string; value: string }>>("get_context_vars", { id: currentJobId });
      const mapped = Object.entries(snapshot).map(([name, info]) => ({
        name,
        type_name: info.type_name,
        value: info.value,
      }));
      setVars(mapped);
    } catch {
      // Ignore — job may have finished or not exist yet.
    }
  }, [currentJobId]);

  // Polling пока джоб активен.
  useEffect(() => {
    if (currentJobId === null) {
      setVars([]);
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      return;
    }

    // Сразу загружаем текущее состояние.
    void fetchVars();
    timerRef.current = setInterval(fetchVars, 400);

    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [currentJobId, fetchVars]);

  const typeBadge = (t: string): "default" | "secondary" | "outline" => {
    switch (t) {
      case "Object":
        return "default";
      case "Number":
        return "secondary";
      case "Boolean":
        return "outline";
      case "String":
        return "secondary";
      default:
        return "outline";
    }
  };

  return (
    <div className="flex max-h-48 flex-col overflow-hidden border-t border-border bg-card">
      <div className="flex items-center gap-2 border-b border-border px-3 py-1.5">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          Контекст
        </h2>
        {currentJobId !== null && vars.length > 0 && (
          <Badge variant="outline" className="text-[10px]">
            {vars.length}
          </Badge>
        )}
      </div>

      {vars.length === 0 ? (
        <p className="px-3 py-2 text-xs text-muted-foreground">
          {currentJobId !== null
            ? "Ожидание данных контекста…"
            : "Запустите робота, чтобы увидеть переменные."}
        </p>
      ) : (
        <ScrollArea className="flex-1">
          <table className="w-full font-mono text-xs">
            <thead>
              <tr className="border-b border-border text-left text-muted-foreground">
                <th className="px-3 py-1">Имя</th>
                <th className="px-3 py-1">Тип</th>
                <th className="px-3 py-1">Значение</th>
                <th className="px-3 py-1"></th>
              </tr>
            </thead>
            <tbody>
              {vars.map((v) => (
                <tr key={v.name} className="border-b border-border/50 hover:bg-secondary/30">
                  <td className="px-3 py-1 font-medium text-foreground">{v.name}</td>
                  <td className="px-3 py-1">
                    <Badge variant={typeBadge(v.type_name)} className="text-[10px]">
                      {v.type_name}
                    </Badge>
                  </td>
                  <td className="max-w-[200px] truncate px-3 py-1 text-muted-foreground">
                    {v.value}
                  </td>
                  <td className="px-3 py-1">
                    {onAddClick && v.type_name === "Object" && (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-5 text-[10px]"
                        onClick={() => onAddClick(v.name)}
                      >
                        + Click
                      </Button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </ScrollArea>
      )}
    </div>
  );
}
