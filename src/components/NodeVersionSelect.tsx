import React from "react";
import { Select, Tag } from "antd";
import { NvmInfo, Project } from "../types/project";

const tagStyle: React.CSSProperties = {
  fontSize: 10,
  lineHeight: 1,
  padding: "2px 5px",
  margin: 0,
  borderRadius: 3,
  verticalAlign: "middle",
};

const labelStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
};

interface NodeVersionSelectProps {
  record: Project;
  nvmInfo: NvmInfo | null;
  onChange: (projectId: string, version: string | null | undefined) => void;
}

const NodeVersionSelect: React.FC<NodeVersionSelectProps> = ({
  record,
  nvmInfo,
  onChange,
}) => {
  if (!nvmInfo?.isInstalled && !nvmInfo?.availableVersions?.length) {
    return (
      <Tag color="default" style={{ cursor: "not-allowed" }}>
        无可用 Node 版本
      </Tag>
    );
  }

  return (
    <Select<string | undefined>
      value={record.nodeVersion || undefined}
      placeholder="选择版本"
      style={{ width: 150 }}
      allowClear
      onChange={(v) => onChange(record.id, v)}
      options={(nvmInfo.availableVersions || []).map((v) => ({
        label: (
          <div style={labelStyle}>
            <span>{v.version}</span>
            {v.isCurrent && (
              <Tag color="green" bordered={false} style={tagStyle}>
                系统
              </Tag>
            )}
            {record.nodeVersion === v.version && (
              <Tag color="blue" bordered={false} style={tagStyle}>
                项目
              </Tag>
            )}
          </div>
        ),
        value: v.version,
      }))}
    />
  );
};

export default NodeVersionSelect;
