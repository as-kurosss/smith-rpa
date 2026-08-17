import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  applyNodeChanges,
  type Edge,
  type Node,
  type NodeChange,
} from "@xyflow/react";
import { ActionNode } from "./components/ActionNode";
import { NodePalette } from "./components/NodePalette";
import { PropertyPanel } from "./components/PropertyPanel";
import { RunLogs } from "./components/RunLogs";
import { Toolbar } from "./components/Toolbar";
import {
  NODE_ROW_HEIGHT,
  makeStepNode,
  nodesToRobot,
  parseRobot,
  robotToNodes,
  serializeRobot,
  sortedNodes,
  stepData,
} from "./lib/robot";
import type { ExecutionReportView, JobView, StepParams, ToolDef } from "./types";

// Типы узлов React Flow (должны быть стабильными ссылками).
const nodeTypes = { action: ActionNode };

const DEFAULT_NAME = "My robot";
const DEFAULT_VERSION = "1.0";

/** Главный экран студии: холст React Flow + палитра + панель свойств. */
export default function App() {
  const [nodes, setNodes] = useState<Node[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [robotName, setRobotName] = useState(DEFAULT_NAME);
  const [version, setVersion] = useState(DEFAULT_VERSION);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [status, setStatus] = useState("Готово");
  const [currentJobId, setCurrentJobId] = useState<number | null>(null);
  const [jobStatus, setJobStatus] = useState<string | null>(null);
  const [report, setReport] = useState<ExecutionReportView | null>(null);
  const [history, setHistory] = useState<JobView[]>([]);
  const [runError, setRunError] = useState<string | null>(null);

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedId) ?? null,
    [nodes, selectedId]
  );

  // Рёбра линейного робота вычисляются из порядка узлов.
  const edges = useMemo<Edge[]>(() => {
    const ordered = sortedNodes(nodes);
    return ordered.slice(0, -1).map((node, i) => ({
      id: `e-${node.id}-${ordered[i + 1].id}`,
      source: node.id,
      target: ordered[i + 1].id,
      animated: true,
    }));
  }, [nodes]);

  const onNodesChange = useCallback((changes: NodeChange[]) => {
    setNodes((nds) => applyNodeChanges(changes, nds));
  }, []);

  const addStep = useCallback((tool: ToolDef) => {
    setNodes((nds) => {
      const ordered = sortedNodes(nds);
      const last = ordered[ordered.length - 1];
      const nextIndex = ordered.length;
      const position = last
        ? { x: last.position.x, y: last.position.y + NODE_ROW_HEIGHT }
        : { x: 40, y: 80 };
      return [...nds, makeStepNode(nextIndex, tool.name, tool.defaultParams, {}, position)];
    });
    setStatus(`Добавлен шаг: ${tool.name}`);
  }, []);

  const updateStep = useCallback(
    (id: string, action: string, params: StepParams, outputs: Record<string, string>) => {
      setNodes((nds) =>
        nds.map((node) =>
          node.id === id
            ? { ...node, data: { ...stepData(node), action, params, outputs } }
            : node
        )
      );
    },
    []
  );

  const moveStep = useCallback((id: string, delta: -1 | 1) => {
    setNodes((nds) => {
      const ordered = sortedNodes(nds);
      const from = ordered.findIndex((node) => node.id === id);
      const to = from + delta;
      if (from < 0 || to < 0 || to >= ordered.length) {
        return nds;
      }
      const a = ordered[from];
      const b = ordered[to];
      return nds.map((node) => {
        if (node.id === a.id) {
          return { ...node, data: { ...stepData(node), index: stepData(b).index } };
        }
        if (node.id === b.id) {
          return { ...node, data: { ...stepData(node), index: stepData(a).index } };
        }
        return node;
      });
    });
  }, []);

  const deleteStep = useCallback((id: string) => {
    setNodes((nds) => {
      const remaining = nds.filter((node) => node.id !== id);
      return sortedNodes(remaining).map((node, i) => ({
        ...node,
        id: `step-${i}`,
        data: { ...stepData(node), index: i },
      }));
    });
    setSelectedId(null);
    setStatus("Шаг удалён");
  }, []);

  const handleSave = useCallback(() => {
    const robot = nodesToRobot(robotName.trim() || DEFAULT_NAME, version, nodes);
    const blob = new Blob([serializeRobot(robot)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = `${robot.name.replace(/[^\wа-яА-Я-]+/g, "_")}.robot.json`;
    anchor.click();
    URL.revokeObjectURL(url);
    setStatus(`Робот сохранён (${robot.steps.length} шагов)`);
  }, [nodes, robotName, version]);

  const handleLoad = useCallback((file: File) => {
    const reader = new FileReader();
    reader.onload = () => {
      try {
        const robot = parseRobot(String(reader.result));
        setRobotName(robot.name);
        setVersion(robot.version);
        setNodes(robotToNodes(robot));
        setSelectedId(null);
        setLoadError(null);
        setStatus(`Робот загружен: ${robot.name} (${robot.steps.length} шагов)`);
      } catch (error) {
        setLoadError(error instanceof Error ? error.message : String(error));
      }
    };
    reader.readAsText(file);
  }, []);

  const isTerminal = (s: string | null): boolean =>
    s === "succeeded" || s === "failed" || s === "cancelled";

  const refreshHistory = useCallback(() => {
    invoke<JobView[]>("get_history")
      .then((jobs) => setHistory(jobs))
      .catch((error: unknown) => setRunError(String(error)));
  }, []);

  const handleRun = useCallback(async () => {
    const robot = nodesToRobot(robotName.trim() || DEFAULT_NAME, version, nodes);
    setRunError(null);
    try {
      const id = await invoke<number>("run_robot", { json: serializeRobot(robot) });
      setCurrentJobId(id);
      setJobStatus("queued");
      setReport(null);
      setStatus(`Робот запущен: #${id}`);
    } catch (error) {
      setRunError(error instanceof Error ? error.message : String(error));
      setStatus("Ошибка запуска");
    }
  }, [nodes, robotName, version]);

  const handleCancel = useCallback(async () => {
    if (currentJobId === null) {
      return;
    }
    try {
      await invoke("cancel_job", { id: currentJobId });
      setStatus(`Отмена запрошена: #${currentJobId}`);
    } catch (error) {
      setRunError(error instanceof Error ? error.message : String(error));
    }
  }, [currentJobId]);

  // Опрос состояния запуска, пока джоб не завершён.
  useEffect(() => {
    if (currentJobId === null) {
      return;
    }
    const timer = window.setInterval(() => {
      invoke<JobView | null>("get_job", { id: currentJobId })
        .then((job) => {
          if (job === null) {
            return;
          }
          setJobStatus(job.status);
          if (job.report) {
            setReport(job.report);
          }
          if (isTerminal(job.status)) {
            setCurrentJobId(null);
            setStatus(`Запуск #${job.id}: ${job.status}`);
            refreshHistory();
          }
        })
        .catch((error: unknown) => setRunError(String(error)));
    }, 400);
    return () => window.clearInterval(timer);
  }, [currentJobId, refreshHistory]);

  // История запусков при старте студии.
  useEffect(() => {
    refreshHistory();
  }, [refreshHistory]);

  return (
    <div className="flex h-full flex-col">
      <Toolbar
        robotName={robotName}
        version={version}
        onNameChange={setRobotName}
        onVersionChange={setVersion}
        onSave={handleSave}
        onLoad={handleLoad}
        onRun={handleRun}
        onCancel={handleCancel}
        canRun={nodes.length > 0}
        running={currentJobId !== null}
      />

      {loadError && (
        <div className="border-b border-red-200 bg-red-50 px-4 py-1 text-sm text-red-700">
          Ошибка загрузки: {loadError}
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <NodePalette onAdd={addStep} />

        <main className="h-full flex-1">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            nodeTypes={nodeTypes}
            onNodeClick={(_, node) => setSelectedId(node.id)}
            onPaneClick={() => setSelectedId(null)}
            fitView
          >
            <Background />
            <Controls />
            <MiniMap pannable zoomable />
          </ReactFlow>
        </main>

        <PropertyPanel
          node={selectedNode}
          onUpdate={updateStep}
          onDelete={deleteStep}
          onMove={moveStep}
        />
      </div>

      {runError && (
        <div className="border-t border-red-200 bg-red-50 px-4 py-1 text-sm text-red-700">
          Ошибка запуска: {runError}
        </div>
      )}

      <RunLogs jobStatus={jobStatus} report={report} history={history} />

      <footer className="border-t border-slate-200 bg-white px-4 py-1 text-xs text-slate-500">
        {status} · {nodes.length} шагов
      </footer>
    </div>
  );
}
