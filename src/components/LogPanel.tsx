import React, { useState, useEffect, useRef } from "react";
import { Collapse, Button, Typography, Empty } from "antd";
import {
  ClearOutlined,
  VerticalAlignBottomOutlined,
} from "@ant-design/icons";
import { tauriAPI } from "../lib/tauri";

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
  const logRef = useRef<HTMLPreElement>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval>>(null);

  useEffect(() => {

    const poll = async () => {
      try {
        const result = await tauriAPI.getScriptOutput(projectId);
        if (result.success && result.data) {
          setOutput(result.data.output || "");
        }
      } catch {
        // silently fail
      }
    };

    poll();
    intervalRef.current = setInterval(poll, 1500);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [projectId]);

  useEffect(() => {
    if (autoScroll && logRef.current) {
      logRef.current.scrollTop = logRef.current.scrollHeight;
    }
  }, [output, autoScroll]);

  return (
    <Collapse
      size="small"
      style={{ marginTop: 16 }}
      items={[
        {
          key: "log",
          label: `脚本输出 - ${projectName}`,
          extra: (
            <span onClick={(e) => e.stopPropagation()}>
              <Button
                size="small"
                type="text"
                icon={<VerticalAlignBottomOutlined />}
                onClick={() => setAutoScroll(!autoScroll)}
                title={autoScroll ? "关闭自动滚动" : "开启自动滚动"}
                style={{ color: autoScroll ? "#0d9488" : undefined }}
              />
              <Button
                size="small"
                type="text"
                icon={<ClearOutlined />}
                onClick={() => setOutput("")}
                title="清空日志"
              />
            </span>
          ),
          children: output ? (
            <pre
              ref={logRef}
              style={{
                maxHeight: 300,
                overflow: "auto",
                background: "#1e1e1e",
                color: "#d4d4d4",
                padding: 12,
                borderRadius: 4,
                fontSize: 12,
                lineHeight: 1.5,
                margin: 0,
                whiteSpace: "pre-wrap",
                wordBreak: "break-all",
              }}
            >
              {output}
            </pre>
          ) : (
            <Empty
              image={Empty.PRESENTED_IMAGE_SIMPLE}
              description={
                <Typography.Text type="secondary">
                  暂无输出，请确保脚本在托管模式下运行
                </Typography.Text>
              }
            />
          ),
        },
      ]}
    />
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
