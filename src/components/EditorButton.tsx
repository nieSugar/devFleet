import React from "react";
import { Tooltip } from "antd";

interface EditorButtonProps {
  icon: string;
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
      <img alt={alt} src={icon} draggable={false} />
    </button>
  </Tooltip>
);

export default EditorButton;
