import React from "react";
import { Select, Tag, Tooltip } from "antd";
import { useTranslation } from "react-i18next";
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
  const { t } = useTranslation();
  const availableVersions = nvmInfo?.availableVersions || [];
  const hasVersionOptions = availableVersions.length > 0;

  if (!hasVersionOptions) {
    if (record.nodeVersion) {
      return (
        <Tooltip title={t("nodeVersion.project")}>
          <Tag color="blue">
            Node {record.nodeVersion}
          </Tag>
        </Tooltip>
      );
    }

    return (
      <Tag color="default" style={{ cursor: "not-allowed" }}>
        {t("nodeVersion.noVersions")}
      </Tag>
    );
  }

  return (
    <Select<string | undefined>
      value={record.nodeVersion || undefined}
      placeholder={t("nodeVersion.selectVersion")}
      allowClear
      popupMatchSelectWidth={false}
      onChange={(v) => onChange(record.id, v)}
      options={availableVersions.map((v) => ({
        label: (
          <div style={labelStyle}>
            <span>{v.version}</span>
            {v.isCurrent && (
              <Tag color="green" bordered={false} style={tagStyle}>
                {t("nodeVersion.system")}
              </Tag>
            )}
            {record.nodeVersion === v.version && (
              <Tag color="blue" bordered={false} style={tagStyle}>
                {t("nodeVersion.project")}
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
