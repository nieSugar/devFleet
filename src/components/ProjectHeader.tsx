import React from "react";
import { Button, Space, Input } from "antd";
import {
  PlusOutlined,
  ReloadOutlined,
  SearchOutlined,
} from "@ant-design/icons";

interface ProjectHeaderProps {
  loading: boolean;
  searchText: string;
  onAdd: () => void;
  onRefresh: () => void;
  onSearch: (text: string) => void;
}

const ProjectHeader: React.FC<ProjectHeaderProps> = ({
  loading,
  searchText,
  onAdd,
  onRefresh,
  onSearch,
}) => (
  <div className="project-manager-header">
    <h2>项目管理</h2>
    <Space>
      <Input
        placeholder="搜索项目"
        prefix={<SearchOutlined />}
        value={searchText}
        onChange={(e) => onSearch(e.target.value)}
        allowClear
        style={{ width: 180 }}
      />
      <Button
        type="primary"
        icon={<PlusOutlined />}
        onClick={onAdd}
        loading={loading}
      >
        添加项目
      </Button>
      <Button icon={<ReloadOutlined />} onClick={onRefresh} disabled={loading}>
        刷新
      </Button>
    </Space>
  </div>
);

export default ProjectHeader;
