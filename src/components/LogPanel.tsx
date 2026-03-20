import React, { useState, useEffect, useRef } from "react";
import {
  ClearOutlined,
  VerticalAlignBottomOutlined,
} from "@ant-design/icons";
import { tauriAPI } from "../lib/tauri";
import "./LogPanel.css";

interface LogPanelProps {
  projectId: string | null;
  projectName: string;
}

const LogPanelInner: React.FC<{ projectId: string; projectName: string }> = ({
  projectId,
  projectName,
}) => {
  const [output, setOutput] = useState("");
  const [autoScroll, setAutoScroll] = useState(true);
  const [collapsed, setCollapsed] = useState(false);
  const logRef = useRef<HTMLPreElement>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval>>(null);

  useEffect(() => {
    const poll = async () => {
      try {
        const result = await tauriAPI.getScriptOutput(projectId);
        if (result.success && result.data) setOutput(result.data.output || "");
      } catch {
        /* silently fail */
      }
    };
    poll();
    intervalRef.current = setInterval(poll, 1500);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [projectId]);

  useEffect(() => {
    if (autoScroll && logRef.current)
      logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [output, autoScroll]);

  return (
    <div className="log-panel">
      <div className="log-header" onClick={() => setCollapsed((c) => !c)}>
        <div className="log-title">
          <span className="log-dot" />
          <span className="log-label">输出</span>
          <span className="log-project-name">{projectName}</span>
        </div>
        <div className="log-controls" onClick={(e) => e.stopPropagation()}>
          <button
            className="log-ctrl-btn"
            onClick={() => setAutoScroll((a) => !a)}
            title={autoScroll ? "关闭自动滚动" : "开启自动滚动"}
          >
            <VerticalAlignBottomOutlined
              style={{ color: autoScroll ? "var(--accent)" : undefined }}
            />
          </button>
          <button
            className="log-ctrl-btn"
            onClick={() => setOutput("")}
            title="清空"
          >
            <ClearOutlined />
          </button>
          <button
            className="log-ctrl-btn"
            onClick={() => setCollapsed((c) => !c)}
          >
            <svg
              width="12"
              height="12"
              viewBox="0 0 12 12"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              style={{
                transform: collapsed ? "rotate(-90deg)" : "rotate(0)",
                transition: "transform 200ms ease",
              }}
            >
              <path d="M3 4.5L6 7.5L9 4.5" />
            </svg>
          </button>
        </div>
      </div>

      {!collapsed && (
        <div className="log-body">
          {output ? (
            <pre ref={logRef} className="log-output">
              {output}
            </pre>
          ) : (
            <div className="log-empty">等待输出...</div>
          )}
        </div>
      )}
    </div>
  );
};

const LogPanel: React.FC<LogPanelProps> = ({ projectId, projectName }) => {
  if (!projectId) return null;
  return (
    <LogPanelInner
      key={projectId}
      projectId={projectId}
      projectName={projectName}
    />
  );
};

export default LogPanel;
