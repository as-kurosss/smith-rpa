import type { ToolDef } from "../types";

/** Каталог инструментов, доступных в студии (совпадает с default_registry движка). */
export const TOOL_CATALOG: ToolDef[] = [
  {
    name: "windows.click",
    label: "Click",
    description: "Клик по элементу (по ключу или селектору)",
    defaultParams: {},
    outputs: [],
  },
  {
    name: "windows.find",
    label: "Find",
    description: "Найти элемент по селектору и сохранить в контекст",
    defaultParams: { output_key: "", name: "" },
    outputs: [],
  },
  {
    name: "windows.input_text",
    label: "Input text",
    description: "Ввод текста в элемент",
    defaultParams: { element_key: "", text: "" },
    outputs: [],
  },
  {
    name: "windows.set_text",
    label: "Set text",
    description: "Установить текст элемента",
    defaultParams: { element_key: "", text: "" },
    outputs: [],
  },
  {
    name: "windows.wait",
    label: "Wait",
    description: "Пауза в миллисекундах",
    defaultParams: { ms: 500 },
    outputs: [],
  },
  {
    name: "windows.process",
    label: "Process",
    description: "Запуск процесса",
    defaultParams: { action: "start", command: "" },
    outputs: [
      {
        name: "pid",
        type: "number",
        description: "PID запущенного процесса",
      },
      {
        name: "status",
        type: "string",
        description: "Статус операции (started/stopped/slept)",
      },
    ],
  },
  {
    name: "windows.extract",
    label: "Extract",
    description: "Прочитать текст (name/value) элемента",
    defaultParams: { property: "name" },
    outputs: [
      {
        name: "text",
        type: "string",
        description: "Извлечённый текст",
      },
    ],
  },
  {
    name: "windows.screenshot",
    label: "Screenshot",
    description: "Снимок экрана или элемента в PNG",
    defaultParams: { path: "" },
    outputs: [
      {
        name: "path",
        type: "string",
        description: "Путь к сохранённому PNG",
      },
    ],
  },
  {
    name: "http.request",
    label: "HTTP request",
    description: "Универсальный HTTP-запрос (включая вызовы LLM API)",
    defaultParams: { url: "", method: "GET" },
    outputs: [
      {
        name: "status",
        type: "number",
        description: "HTTP-статус ответа",
      },
      {
        name: "body",
        type: "object",
        description: "Тело ответа (JSON или строка)",
      },
    ],
  },
];
