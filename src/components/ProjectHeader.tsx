import React from "react";
import { Button, Input, Tooltip } from "antd";
import {
  PlusOutlined,
  ReloadOutlined,
  SearchOutlined,
} from "@ant-design/icons";
import { useTranslation } from "react-i18next";

interface ProjectHeaderProps {
  loading: boolean;
  projectCount: number;
  totalCount: number;
  searchText: string;
  onAdd: () => void;
  onRefresh: () => void;
  onSearch: (text: string) => void;
}

const ProjectHeader: React.FC<ProjectHeaderProps> = ({
  loading,
  projectCount,
  totalCount,
  searchText,
  onAdd,
  onRefresh,
  onSearch,
}) => {
  const { t } = useTranslation();
  return (
    <div className="project-header">
      <div className="header-left">
        <h1 className="header-title">{t("project.title")}</h1>
        <span className="header-count">
          {projectCount}
          {searchText && totalCount !== projectCount ? ` / ${totalCount}` : ""}
        </span>
      </div>
      <div className="header-right">
        <Input
          placeholder={t("project.searchPlaceholder")}
          prefix={<SearchOutlined style={{ color: "var(--text-muted)" }} />}
          value={searchText}
          onChange={(e) => onSearch(e.target.value)}
          allowClear
          className="header-search"
        />
        <Tooltip title={t("project.refreshTooltip")}>
          <Button
            icon={<ReloadOutlined spin={loading} />}
            onClick={onRefresh}
            disabled={loading}
          />
        </Tooltip>
        <Button
          type="primary"
          ghost
          icon={<PlusOutlined />}
          onClick={onAdd}
        >
          {t("common.add")}
        </Button>
      </div>
    </div>
  );
};

export default ProjectHeader;
