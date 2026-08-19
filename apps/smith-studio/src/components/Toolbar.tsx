import { useRef } from "react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Label } from "./ui/label";

interface ToolbarProps {
  robotName: string;
  version: string;
  onNameChange: (name: string) => void;
  onVersionChange: (version: string) => void;
  onSave: () => void;
  onLoad: (file: File) => void;
  onRun: () => void;
  onCancel: () => void;
  onDebugRun: () => void;
  onResume: () => void;
  onStepOver: () => void;
  canRun: boolean;
  running: boolean;
  debugMode: boolean;
  isPaused: boolean;
}

/** Верхняя панель: имя робота, версия, сохранение/загрузка JSON, запуск. */
export function Toolbar({
  robotName,
  version,
  onNameChange,
  onVersionChange,
  onSave,
  onLoad,
  onRun,
  onCancel,
  onDebugRun,
  onResume,
  onStepOver,
  canRun,
  running,
  debugMode,
  isPaused,
}: ToolbarProps) {
  const fileRef = useRef<HTMLInputElement>(null);

  const handleFileChange = (file: File | undefined) => {
    if (file) {
      onLoad(file);
    }
    if (fileRef.current) {
      fileRef.current.value = "";
    }
  };

  return (
    <header className="flex items-center gap-3 border-b border-border bg-card px-4 py-2">
      <h1 className="mr-2 text-sm font-bold text-foreground">Smith RPA Studio</h1>

      <Label className="text-muted-foreground">Имя</Label>
      <Input
        value={robotName}
        onChange={(e) => onNameChange(e.target.value)}
        className="w-44"
        placeholder="My robot"
      />

      <Label className="text-muted-foreground">Версия</Label>
      <Input
        value={version}
        onChange={(e) => onVersionChange(e.target.value)}
        className="w-20"
      />

      <span className="ml-auto flex items-center gap-2">
        <input
          ref={fileRef}
          type="file"
          accept=".json,application/json"
          className="hidden"
          onChange={(e) => handleFileChange(e.target.files?.[0])}
        />
        <Button variant="outline" size="sm" onClick={() => fileRef.current?.click()}>
          Загрузить JSON
        </Button>
        <Button variant="outline" size="sm" onClick={onSave}>
          Сохранить JSON
        </Button>
        {debugMode && isPaused && (
          <>
            <Button variant="outline" size="sm" onClick={onResume}>
              ▶ Продолжить
            </Button>
            <Button variant="outline" size="sm" onClick={onStepOver}>
              ⏭ Шаг
            </Button>
          </>
        )}
        {running && !debugMode && (
          <Button variant="destructive" size="sm" onClick={onCancel}>
            Отменить
          </Button>
        )}
        {debugMode && running && (
          <Button variant="destructive" size="sm" onClick={onCancel}>
            Стоп
          </Button>
        )}
        {!debugMode && (
          <Button
            size="sm"
            onClick={onRun}
            disabled={!canRun || running}
            className={running ? "bg-amber-500 hover:bg-amber-600" : ""}
          >
            {running ? "Выполняется…" : "Запустить"}
          </Button>
        )}
        {!running && (
          <Button
            size="sm"
            variant="outline"
            onClick={onDebugRun}
            disabled={!canRun}
          >
            ▶ Пошагово
          </Button>
        )}
      </span>
    </header>
  );
}
