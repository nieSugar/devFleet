import React from "react";
import { Button, Input, Tooltip } from "antd";
import {
  PlusOutlined,
  ReloadOutlined,
  SearchOutlined,
} from "@ant-design/icons";

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
}) => (
  <div className="project-header">
    <div className="header-left">
      <h1 className="header-title">项目</h1>
      <span className="header-count">
        {projectCount}
        {searchText && totalCount !== projectCount ? ` / ${totalCount}` : ""}
      </span>
    </div>
    <div className="header-right">
      <Input
        placeholder="搜索项目..."
        prefix={<SearchOutlined style={{ color: "var(--text-muted)" }} />}
        value={searchText}
        onChange={(e) => onSearch(e.target.value)}
        allowClear
        className="header-search"
      />
      <Tooltip title="刷新 (Ctrl+R)">
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
        添加
      </Button>
    </div>
  </div>
);

export default ProjectHeader;
