import React from "react";
import { Button, Input, Select, Tooltip, type InputRef } from "antd";
import {
  PlusOutlined,
  ReloadOutlined,
  SearchOutlined,
  AppstoreOutlined,
  UnorderedListOutlined,
} from "@ant-design/icons";
import { useTranslation } from "react-i18next";

interface ProjectHeaderProps {
  loading: boolean;
  projectCount: number;
  totalCount: number;
  searchText: string;
  searchRef: React.Ref<InputRef>;
  onSearchKeyDown: React.KeyboardEventHandler<HTMLInputElement>;
  onSearchFocus: () => void;
  onSearchBlur: () => void;
  projectFilter: "all" | "processes" | "unavailable";
  onFilterChange: (filter: "all" | "processes" | "unavailable") => void;
  compact: boolean;
  onCompactChange: (compact: boolean) => void;
  onAdd: () => void;
  onImport: () => void;
  onRefresh: () => void;
  onSearch: (text: string) => void;
}

const ProjectHeader: React.FC<ProjectHeaderProps> = ({
  loading,
  projectCount,
  totalCount,
  searchText,
  searchRef,
  onSearchKeyDown,
  onSearchFocus,
  onSearchBlur,
  projectFilter,
  onFilterChange,
  compact,
  onCompactChange,
  onAdd,
  onImport,
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
          {totalCount !== projectCount ? ` / ${totalCount}` : ""}
        </span>
      </div>
      <div className="header-right">
        <Input
          ref={searchRef}
          aria-label={t("project.searchPlaceholder")}
          aria-controls="project-results"
          aria-describedby="project-search-help project-search-selection"
          title={t("project.searchKeyboardHelp")}
          placeholder={t("project.searchPlaceholder")}
          prefix={<SearchOutlined style={{ color: "var(--text-muted)" }} />}
          value={searchText}
          onChange={(e) => onSearch(e.target.value)}
          onKeyDown={onSearchKeyDown}
          onFocus={onSearchFocus}
          onBlur={onSearchBlur}
          allowClear
          className="header-search"
        />
        <span className="project-sr-only" id="project-search-help">{t("project.searchKeyboardHelp")}</span>
        <Select
          value={projectFilter}
          onChange={onFilterChange}
          aria-label={t("project.filterLabel")}
          className="header-filter"
          popupMatchSelectWidth={false}
          options={[
            { value: "all", label: t("project.filterAll") },
            { value: "processes", label: t("project.filterProcesses") },
            { value: "unavailable", label: t("project.filterUnavailable") },
          ]}
        />
        <Tooltip title={t(compact ? "project.cardView" : "project.compactView")}>
          <Button
            icon={compact ? <AppstoreOutlined /> : <UnorderedListOutlined />}
            aria-label={t("project.compactView")}
            aria-pressed={compact}
            onClick={() => onCompactChange(!compact)}
          />
        </Tooltip>
        <Tooltip title={t("project.refreshTooltip")}>
          <Button
            icon={<ReloadOutlined spin={loading} />}
            onClick={onRefresh}
            disabled={loading}
          />
        </Tooltip>
        <Button onClick={onImport}>
          {t("project.import.button")}
        </Button>
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
