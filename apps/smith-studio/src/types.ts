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
