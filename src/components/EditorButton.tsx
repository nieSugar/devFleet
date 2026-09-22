import React, { useState } from "react";
import { Tooltip } from "antd";

interface EditorButtonProps {
  icon?: string;
  alt: string;
  title: string;
  onClick: () => void;
}

const EditorButton: React.FC<EditorButtonProps> = ({ icon, alt, title, onClick }) => {
  const [failedIcon, setFailedIcon] = useState<string | null>(null);

  const fallback =
    alt
      .trim()
      .split(/\s+/)
      .map((part) => part[0])
      .join("")
      .slice(0, 2)
      .toUpperCase() || "IDE";

  return (
    <Tooltip title={title} placement="top" mouseEnterDelay={0.4}>
      <button className="editor-btn" onClick={onClick} aria-label={title}>
        {icon && failedIcon !== icon ? (
          <img
            alt={alt}
            src={icon}
            draggable={false}
            onError={() => setFailedIcon(icon)}
          />
        ) : (
          <span className="editor-btn-text" aria-hidden="true">
            {fallback}
          </span>
        )}
      </button>
    </Tooltip>
  );
};

export default EditorButton;
