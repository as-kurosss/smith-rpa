import type { StepParams } from "../types";
import { Input } from "./ui/input";
import { Label } from "./ui/label";
import { Textarea } from "./ui/textarea";

// ---------------------------------------------------------------------------
// Field descriptors
// ---------------------------------------------------------------------------

type FieldDef = {
  key: string;
  label: string;
  type: "text" | "number" | "select" | "textarea" | "checkbox";
  placeholder?: string;
  options?: string[];
  required?: boolean;
  /** Show only when another param equals this value. */
  showWhen?: { key: string; value: unknown };
};

const CONTROL_TYPES = [
  "",
  "Button",
  "Edit",
  "Text",
  "Window",
  "Image",
  "CheckBox",
  "ComboBox",
  "List",
  "ListItem",
  "MenuItem",
  "Hyperlink",
  "DataItem",
  "ToolBar",
  "StatusBar",
  "Tab",
  "TabItem",
  "Tree",
  "TreeItem",
  "Pane",
  "Document",
  "Group",
  "Thumb",
  "Table",
  "TableItem",
];

const PROCESS_ACTIONS = ["start", "stop", "sleep"];

// Selector fields shared by tools that resolve elements.
const SELECTOR_FIELDS: FieldDef[] = [
  { key: "element_key", label: "Ключ элемента", type: "text", placeholder: "my_element" },
  { key: "name", label: "Имя элемента", type: "text", placeholder: "OK" },
  { key: "automation_id", label: "Automation ID", type: "text" },
  { key: "control_type", label: "Тип контрола", type: "select", options: CONTROL_TYPES },
  { key: "class_name", label: "Class Name", type: "text" },
];

const DELAY_FIELDS: FieldDef[] = [
  { key: "delay_before_ms", label: "Задержка до (мс)", type: "number" },
  { key: "delay_after_ms", label: "Задержка после (мс)", type: "number" },
];

// ---------------------------------------------------------------------------
// Tool-specific schemas
// ---------------------------------------------------------------------------

const TOOL_SCHEMAS: Record<string, FieldDef[]> = {
  "windows.find": [
    { key: "output_key", label: "Ключ контекста", type: "text", placeholder: "my_element", required: true },
    { key: "name", label: "Имя элемента", type: "text", placeholder: "OK" },
    { key: "automation_id", label: "Automation ID", type: "text" },
    { key: "control_type", label: "Тип контрола", type: "select", options: CONTROL_TYPES },
    { key: "class_name", label: "Class Name", type: "text" },
    { key: "pid", label: "PID процесса", type: "number" },
    ...DELAY_FIELDS,
  ],

  "windows.click": [
    ...SELECTOR_FIELDS,
    ...DELAY_FIELDS,
  ],

  "windows.input_text": [
    { key: "text", label: "Текст", type: "text", placeholder: "Hello world", required: true },
    ...SELECTOR_FIELDS,
    ...DELAY_FIELDS,
  ],

  "windows.set_text": [
    { key: "text", label: "Текст", type: "text", placeholder: "Hello world", required: true },
    ...SELECTOR_FIELDS,
    ...DELAY_FIELDS,
  ],

  "windows.wait": [
    { key: "ms", label: "Миллисекунды", type: "number", required: true },
  ],

  "windows.process": [
    { key: "action", label: "Действие", type: "select", options: PROCESS_ACTIONS, required: true },
    { key: "command", label: "Команда", type: "text", placeholder: "notepad.exe", showWhen: { key: "action", value: "start" } },
    { key: "args", label: "Аргументы", type: "text", placeholder: "--arg1 val1", showWhen: { key: "action", value: "start" } },
    { key: "working_dir", label: "Рабочая папка", type: "text", showWhen: { key: "action", value: "start" } },
    { key: "pid", label: "PID", type: "number", showWhen: { key: "action", value: "stop" } },
    { key: "name", label: "Имя процесса", type: "text", placeholder: "notepad.exe", showWhen: { key: "action", value: "stop" } },
    { key: "duration_ms", label: "Длительность (мс)", type: "number", showWhen: { key: "action", value: "sleep" } },
    ...DELAY_FIELDS,
  ],

  "windows.extract": [
    ...SELECTOR_FIELDS,
    { key: "property", label: "Свойство", type: "select", options: ["", "name", "value"] },
  ],

  "windows.screenshot": [
    { key: "path", label: "Путь к PNG", type: "text", placeholder: "C:\\screenshots\\shot.png", required: true },
    ...SELECTOR_FIELDS,
  ],

  "http.request": [
    { key: "url", label: "URL", type: "text", placeholder: "https://api.example.com/v1", required: true },
    { key: "method", label: "Метод", type: "select", options: ["GET", "POST", "PUT", "DELETE", "PATCH"] },
    { key: "body", label: "Тело (JSON)", type: "textarea", placeholder: '{"key": "value"}' },
    { key: "headers", label: "Заголовки (JSON)", type: "textarea", placeholder: '{"Authorization": "Bearer ..."}' },
    { key: "timeout_ms", label: "Таймаут (мс)", type: "number" },
  ],
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface ToolParamFormProps {
  toolName: string;
  params: StepParams;
  onChange: (params: StepParams) => void;
}

export function ToolParamForm({ toolName, params, onChange }: ToolParamFormProps) {
  const fields = TOOL_SCHEMAS[toolName];

  // Fallback: unknown tool → show raw JSON
  if (!fields) {
    return null; // PropertyPanel will show JSON textarea
  }

  const update = (key: string, value: unknown) => {
    const next = { ...params };
    if (value === "" || value === undefined || value === null) {
      delete next[key];
    } else if (key === "args" && typeof value === "string") {
      // Split comma-separated args into array for the Rust tool.
      const parts = value.split(/\s*,\s*/).filter(Boolean);
      next[key] = parts.length > 0 ? parts : undefined;
      if (parts.length === 0) delete next[key];
    } else if (key === "headers" && typeof value === "string") {
      try {
        next[key] = JSON.parse(value);
      } catch {
        next[key] = value;
      }
    } else if (key === "body" && typeof value === "string") {
      try {
        next[key] = JSON.parse(value);
      } catch {
        next[key] = value;
      }
    } else {
      next[key] = value;
    }
    onChange(next);
  };

  return (
    <div className="space-y-3">
      {fields.map((field) => {
        // Conditional visibility
        if (field.showWhen) {
          const triggerValue = params[field.showWhen.key];
          if (triggerValue !== field.showWhen.value) return null;
        }

        const rawValue = params[field.key] ?? "";
        let strValue: string;
        if (field.key === "args" && Array.isArray(rawValue)) {
          strValue = rawValue.join(", ");
        } else if ((field.key === "headers" || field.key === "body") && typeof rawValue === "object") {
          strValue = JSON.stringify(rawValue, null, 2);
        } else {
          strValue = typeof rawValue === "string" ? rawValue : String(rawValue);
        }

        return (
          <div key={field.key}>
            <Label className="mb-0.5 flex items-center gap-1 text-xs">
              {field.label}
              {field.required && <span className="text-destructive">*</span>}
            </Label>

            {field.type === "select" ? (
              <select
                value={strValue}
                onChange={(e) => update(field.key, e.target.value)}
                className="flex h-9 w-full rounded-md border border-input bg-transparent px-3 py-1 text-xs shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring font-mono"
              >
                {(field.options ?? []).map((opt) => (
                  <option key={opt} value={opt}>
                    {opt || "— не задано —"}
                  </option>
                ))}
              </select>
            ) : field.type === "textarea" ? (
              <Textarea
                value={strValue}
                onChange={(e) => update(field.key, e.target.value)}
                rows={3}
                spellCheck={false}
                placeholder={field.placeholder}
                className="font-mono text-xs"
              />
            ) : field.type === "number" ? (
              <Input
                type="number"
                value={strValue}
                onChange={(e) => update(field.key, e.target.value === "" ? "" : Number(e.target.value))}
                placeholder={field.placeholder}
                className="font-mono text-xs"
              />
            ) : (
              <Input
                type="text"
                value={strValue}
                onChange={(e) => update(field.key, e.target.value)}
                placeholder={field.placeholder}
                className="font-mono text-xs"
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

/** Returns true if the tool has a known schema (i.e. form is available). */
export function hasToolForm(toolName: string): boolean {
  return toolName in TOOL_SCHEMAS;
}
