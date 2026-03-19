import React from "react";
import { Select, Space, Tag } from "antd";
import { NvmInfo, Project } from "../types/project";

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
  if (!nvmInfo?.isInstalled) {
    return (
      <Tag color="default" style={{ cursor: "not-allowed" }}>
        未安装版本管理器
      </Tag>
    );
  }

  return (
    <Select<string | undefined>
      value={record.nodeVersion || undefined}
      placeholder="选择版本"
      style={{ width: 140 }}
      allowClear
      onChange={(v) => onChange(record.id, v)}
      options={(nvmInfo.availableVersions || []).map((v) => ({
        label: (
          <Space size={4}>
            {v.version}
            {v.isCurrent && (
              <Tag color="green" style={{ fontSize: 10, padding: "0 4px" }}>
                系统
              </Tag>
            )}
            {record.nodeVersion === v.version && (
              <Tag color="blue" style={{ fontSize: 10, padding: "0 4px" }}>
                项目
              </Tag>
            )}
          </Space>
        ),
        value: v.version,
      }))}
    />
  );
};

export default NodeVersionSelect;
