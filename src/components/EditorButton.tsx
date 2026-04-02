import React from "react";
import { Tooltip } from "antd";

interface EditorButtonProps {
  icon?: React.ReactNode;
  alt: string;
  title: string;
  onClick: () => void;
}

const EditorButton: React.FC<EditorButtonProps> = ({
  icon,
  alt,
  title,
  onClick,
}) => (
  <Tooltip title={title} placement="top" mouseEnterDelay={0.4}>
    <button className="editor-btn" onClick={onClick} aria-label={title}>
      {icon || <span className="editor-btn-text">{alt}</span>}
    </button>
  </Tooltip>
);

export default EditorButton;
