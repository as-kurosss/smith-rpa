/** Параметры шага — произвольный JSON-объект. */
export type StepParams = Record<string, unknown>;

/** Описание выходного поля инструмента. */
export interface OutputDef {
  name: string;
  type: string;
  description: string;
}

/** Один шаг робота: вызов инструмента с параметрами. */
export interface RobotStep {
  action: string;
  params: StepParams;
  /** Маппинг выходов инструмента в переменные контекста: поле -> имя переменной. */
  outputs?: Record<string, string>;
}

/** Модель робота (JSON-формат smith-engine). */
export interface RobotModel {
  name: string;
  version: string;
  steps: RobotStep[];
}

/** Данные узла на холсте React Flow. */
export interface ActionNodeData {
  index: number;
  action: string;
  params: StepParams;
  /** Маппинг выходов: поле инструмента -> имя переменной контекста. */
  outputs: Record<string, string>;
}

/** Описание инструмента для палитры узлов. */
export interface ToolDef {
  name: string;
  label: string;
  description: string;
  defaultParams: StepParams;
  /** Выходные поля инструмента (для маппинга в переменные). */
  outputs: OutputDef[];
}

/** Результат одного шага из ExecutionReport. */
export interface StepLog {
  action: string;
  ok: boolean;
  output?: unknown;
  error?: string;
}

/** Итоговый отчёт о запуске (smith-engine ExecutionReport). */
export interface ExecutionReportView {
  robot_name: string;
  status: string;
  steps: StepLog[];
}

/** Запись запуска из smith-orchestrator (Job). */
export interface JobView {
  id: number;
  robot_name: string;
  status: string;
  report: ExecutionReportView | null;
}
