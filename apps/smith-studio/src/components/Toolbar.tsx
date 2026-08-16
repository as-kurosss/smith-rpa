import { useRef } from "react";

interface ToolbarProps {
  robotName: string;
  version: string;
  onNameChange: (name: string) => void;
  onVersionChange: (version: string) => void;
  onSave: () => void;
  onLoad: (file: File) => void;
  onRun: () => void;
  onCancel: () => void;
  canRun: boolean;
  running: boolean;
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
  canRun,
  running,
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
    <header className="flex items-center gap-3 border-b border-slate-200 bg-white px-4 py-2">
      <h1 className="mr-2 text-sm font-bold text-slate-800">Smith RPA Studio</h1>

      <label className="text-xs text-slate-500">Имя</label>
      <input
        value={robotName}
        onChange={(e) => onNameChange(e.target.value)}
        className="w-44 rounded-md border border-slate-300 px-2 py-1 text-sm"
        placeholder="My robot"
      />

      <label className="text-xs text-slate-500">Версия</label>
      <input
        value={version}
        onChange={(e) => onVersionChange(e.target.value)}
        className="w-20 rounded-md border border-slate-300 px-2 py-1 text-sm"
      />

      <span className="ml-auto flex items-center gap-2">
        <input
          ref={fileRef}
          type="file"
          accept=".json,application/json"
          className="hidden"
          onChange={(e) => handleFileChange(e.target.files?.[0])}
        />
        <button
          type="button"
          onClick={() => fileRef.current?.click()}
          className="rounded-md border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-50"
        >
          Загрузить JSON
        </button>
        <button
          type="button"
          onClick={onSave}
          className="rounded-md border border-slate-300 px-3 py-1.5 text-sm hover:bg-slate-50"
        >
          Сохранить JSON
        </button>
        {running && (
          <button
            type="button"
            onClick={onCancel}
            className="rounded-md border border-red-300 bg-red-50 px-3 py-1.5 text-sm text-red-700 hover:bg-red-100"
          >
            Отменить
          </button>
        )}
        <button
          type="button"
          onClick={onRun}
          disabled={!canRun || running}
          className={`rounded-md px-4 py-1.5 text-sm font-medium text-white ${
            running ? "bg-amber-500" : "bg-blue-600 hover:bg-blue-700 disabled:opacity-40"
          }`}
        >
          {running ? "Выполняется…" : "Запустить"}
        </button>
      </span>
    </header>
  );
}
