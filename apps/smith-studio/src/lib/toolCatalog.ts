import type { ToolDef } from "../types";

/** Каталог инструментов, доступных в студии (совпадает с default_registry движка). */
export const TOOL_CATALOG: ToolDef[] = [
  {
    name: "windows.click",
    label: "Click",
    description: "Клик по элементу, сохранённому в контексте",
    defaultParams: { element_key: "" },
  },
  {
    name: "windows.find",
    label: "Find",
    description: "Найти элемент по селектору и сохранить в контекст",
    defaultParams: { selector: "" },
  },
  {
    name: "windows.input_text",
    label: "Input text",
    description: "Ввод текста в элемент",
    defaultParams: { element_key: "", text: "" },
  },
  {
    name: "windows.set_text",
    label: "Set text",
    description: "Установить текст элемента",
    defaultParams: { element_key: "", text: "" },
  },
  {
    name: "windows.wait",
    label: "Wait",
    description: "Пауза в миллисекундах",
    defaultParams: { ms: 500 },
  },
  {
    name: "windows.process",
    label: "Process",
    description: "Запуск процесса",
    defaultParams: { path: "" },
  },
  {
    name: "windows.extract",
    label: "Extract",
    description: "Прочитать текст (name/value) элемента",
    defaultParams: { element_key: "", property: "name" },
  },
  {
    name: "windows.screenshot",
    label: "Screenshot",
    description: "Снимок экрана или элемента в PNG",
    defaultParams: { path: "" },
  },
  {
    name: "http.request",
    label: "HTTP request",
    description: "Универсальный HTTP-запрос (включая вызовы LLM API)",
    defaultParams: { url: "", method: "GET" },
  },
];
