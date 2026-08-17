import type { Node } from "@xyflow/react";
import type { ActionNodeData, RobotModel, RobotStep, StepParams } from "../types";

/** Вертикальный отступ между узлами при авто-раскладке. */
export const NODE_ROW_HEIGHT = 85;

/** Создаёт узел шага робота. */
export function makeStepNode(
  index: number,
  action: string,
  params: StepParams,
  outputs: Record<string, string>,
  position: { x: number; y: number }
): Node {
  return {
    id: `step-${index}`,
    type: "action",
    position,
    data: { index, action, params, outputs } satisfies ActionNodeData,
  };
}

/** Данные узла с типизацией. */
export function stepData(node: Node): ActionNodeData {
  return node.data as unknown as ActionNodeData;
}

/** Сортирует узлы по индексу шага (порядок исполнения). */
export function sortedNodes(nodes: Node[]): Node[] {
  return [...nodes].sort((a, b) => stepData(a).index - stepData(b).index);
}

/** Робот -> узлы (вертикальная раскладка). */
export function robotToNodes(robot: RobotModel): Node[] {
  return robot.steps.map((step, index) =>
    makeStepNode(index, step.action, step.params, step.outputs ?? {}, {
      x: 40,
      y: 80 + index * NODE_ROW_HEIGHT,
    })
  );
}

/** Узлы -> робот (по порядку индексов). */
export function nodesToRobot(name: string, version: string, nodes: Node[]): RobotModel {
  const steps: RobotStep[] = sortedNodes(nodes).map((node) => {
    const data = stepData(node);
    const step: RobotStep = { action: data.action, params: data.params };
    if (Object.keys(data.outputs).length > 0) {
      step.outputs = data.outputs;
    }
    return step;
  });
  return { name, version, steps };
}

/** Робот -> pretty-printed JSON. */
export function serializeRobot(robot: RobotModel): string {
  return JSON.stringify(robot, null, 2);
}

/** Парсит JSON робота; бросает Error с понятным сообщением. */
export function parseRobot(json: string): RobotModel {
  const parsed: unknown = JSON.parse(json);
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error("Робот должен быть JSON-объектом");
  }
  const obj = parsed as Record<string, unknown>;
  if (typeof obj.name !== "string") {
    throw new Error("Отсутствует строковое поле 'name'");
  }
  if (typeof obj.version !== "string") {
    throw new Error("Отсутствует строковое поле 'version'");
  }
  if (!Array.isArray(obj.steps)) {
    throw new Error("Отсутствует массив 'steps'");
  }
  const steps: RobotStep[] = (obj.steps as unknown[]).map((raw, i) => {
    const step = raw as Record<string, unknown>;
    if (typeof step.action !== "string") {
      throw new Error(`steps[${i}].action должен быть строкой`);
    }
    const params =
      typeof step.params === "object" && step.params !== null
        ? (step.params as StepParams)
        : {};
    const outputs =
      typeof step.outputs === "object" && step.outputs !== null
        ? (step.outputs as Record<string, string>)
        : {};
    return { action: step.action, params, outputs };
  });
  return { name: obj.name, version: obj.version, steps };
}
