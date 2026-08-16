/** Параметры шага — произвольный JSON-объект. */
export type StepParams = Record<string, unknown>;

/** Один шаг робота: вызов инструмента с параметрами. */
export interface RobotStep {
  action: string;
  params: StepParams;
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
}

/** Описание инструмента для палитры узлов. */
export interface ToolDef {
  name: string;
  label: string;
  description: string;
  defaultParams: StepParams;
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
