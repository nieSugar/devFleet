import React from "react";
import { Button } from "antd";

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
  <Button size="small" type="text" title={title} aria-label={title} onClick={onClick}>
    <img alt={alt} src={icon} style={{ width: 16, height: 16 }} />
  </Button>
);

export default EditorButton;
