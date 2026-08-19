import { useEffect, useState } from "react";
import type { Node } from "@xyflow/react";
import { stepData } from "../lib/robot";
import { TOOL_CATALOG } from "../lib/toolCatalog";
import type { StepParams } from "../types";
import { ToolParamForm, hasToolForm } from "./ToolParamForm";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Textarea } from "./ui/textarea";

interface PropertyPanelProps {
  node: Node | null;
  onUpdate: (
    id: string,
    action: string,
    params: StepParams,
    outputs: Record<string, string>
  ) => void;
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
      <aside className="w-80 border-l border-border bg-card p-4 text-sm text-muted-foreground">
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
      onUpdate(node.id, data.action, parsed as StepParams, data.outputs);
    } catch (error) {
      setParamsError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleActionChange = (action: string) => {
    onUpdate(node.id, action, data.params, data.outputs);
  };

  const tool = TOOL_CATALOG.find((t) => t.name === data.action);
  const outputDefs = tool?.outputs ?? [];

  const handleOutputChange = (field: string, varName: string) => {
    const outputs = { ...data.outputs };
    if (varName.trim() === "") {
      delete outputs[field];
    } else {
      outputs[field] = varName.trim();
    }
    onUpdate(node.id, data.action, data.params, outputs);
  };

  return (
    <aside className="w-80 overflow-y-auto border-l border-border bg-card p-4">
      <h2 className="mb-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
        Шаг {data.index + 1}
      </h2>

      <Label className="mb-1">Инструмент</Label>
      <Input
        value={data.action}
        onChange={(e) => handleActionChange(e.target.value)}
        className="mb-4 font-mono"
      />

      {hasToolForm(data.action) ? (
        <>
          <ToolParamForm
            toolName={data.action}
            params={data.params}
            onChange={(p) => onUpdate(node.id, data.action, p, data.outputs)}
          />
          <details className="mt-2">
            <summary className="cursor-pointer text-xs text-muted-foreground hover:text-foreground">
              JSON (расширенный)
            </summary>
            <Textarea
              value={paramsText}
              onChange={(e) => handleParamsChange(e.target.value)}
              rows={6}
              spellCheck={false}
              className={`mt-1 font-mono text-xs ${paramsError ? "border-destructive" : ""}`}
            />
            {paramsError && <p className="mt-1 text-xs text-destructive">{paramsError}</p>}
          </details>
        </>
      ) : (
        <>
          <Label className="mb-1">Параметры (JSON)</Label>
          <Textarea
            value={paramsText}
            onChange={(e) => handleParamsChange(e.target.value)}
            rows={12}
            spellCheck={false}
            className={`font-mono text-xs ${paramsError ? "border-destructive" : ""}`}
          />
          {paramsError && <p className="mt-1 text-xs text-destructive">{paramsError}</p>}
        </>
      )}

      <div className="mb-4">
        <Label className="mb-1">Выходы</Label>
        {outputDefs.length === 0 ? (
          <p className="text-xs text-muted-foreground">
            Этот инструмент не возвращает значений.
          </p>
        ) : (
          <div className="space-y-2">
            {outputDefs.map((output) => (
              <div key={output.name}>
                <Label className="mb-0.5 font-mono text-xs text-muted-foreground">
                  {output.name}{" "}
                  <span className="text-muted-foreground">({output.type})</span>
                </Label>
                <Input
                  value={data.outputs[output.name] ?? ""}
                  onChange={(e) => handleOutputChange(output.name, e.target.value)}
                  placeholder="имя переменной"
                  className="font-mono text-xs"
                />
                <p className="mt-0.5 text-[10px] text-muted-foreground">{output.description}</p>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="mt-4 flex gap-2">
        <Button variant="outline" size="sm" onClick={() => onMove(node.id, -1)} className="flex-1">
          ↑ Вверх
        </Button>
        <Button variant="outline" size="sm" onClick={() => onMove(node.id, 1)} className="flex-1">
          ↓ Вниз
        </Button>
      </div>
      <Button
        variant="destructive"
        size="sm"
        onClick={() => onDelete(node.id)}
        className="mt-2 w-full"
      >
        Удалить шаг
      </Button>
    </aside>
  );
}
