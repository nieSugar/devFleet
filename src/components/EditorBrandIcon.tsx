import React from "react";

interface EditorBrandIconProps {
  id: string;
  size?: number;
}

const commonSvgProps = (size: number): React.SVGProps<SVGSVGElement> => ({
  width: size,
  height: size,
  viewBox: "0 0 16 16",
  "aria-hidden": true,
  focusable: false,
});

const EDITOR_BRAND_ICONS: Record<string, (size: number) => React.ReactNode> = {
  cursor: (size) => (
    <svg {...commonSvgProps(size)}>
      <rect width="16" height="16" rx="4" fill="#101114" />
      <path
        d="M4.3 2.8 11.8 8 8.6 8.6l1.7 3.5-1.9.9-1.7-3.5-2.4 2.4V2.8Z"
        fill="#fff"
      />
    </svg>
  ),
  windsurf: (size) => (
    <svg {...commonSvgProps(size)}>
      <rect width="16" height="16" rx="4" fill="#0f766e" />
      <path
        d="M3 6.1c1.5-1.8 3.4-2.2 5.4-1.1 1.2.7 2.4.6 3.6-.4.3-.2.7.1.5.5-1.1 2.3-3 3.1-5 2-1.4-.8-2.7-.7-4 .4-.4.3-.8-.1-.5-.5Z"
        fill="#ccfbf1"
      />
      <path
        d="M3.2 10.3c1.5-1.3 3-1.5 4.6-.6 1.5.8 3 .5 4.4-.8.3-.3.8 0 .6.4-.9 2.1-2.9 3-5 2-1.5-.7-2.7-.5-4 .5-.4.3-.9-.2-.6-.5Z"
        fill="#5eead4"
      />
    </svg>
  ),
  trae: (size) => (
    <svg {...commonSvgProps(size)}>
      <rect width="16" height="16" rx="4" fill="#1f1b2d" />
      <path d="M3.2 4h9.6v2.1H9.1V12H6.9V6.1H3.2V4Z" fill="#a78bfa" />
      <circle cx="11.8" cy="11.6" r="1.1" fill="#f472b6" />
    </svg>
  ),
  antigravity: (size) => (
    <svg {...commonSvgProps(size)}>
      <rect width="16" height="16" rx="4" fill="#111827" />
      <path
        d="M3.2 9.5c2.1 2.6 7.5 2.3 9.7-.6"
        fill="none"
        stroke="#34d399"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <path
        d="M4.1 5.7c2.3-2.2 6.3-1.7 8 1.1"
        fill="none"
        stroke="#60a5fa"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
      <circle cx="8" cy="8" r="2" fill="#f9fafb" />
      <circle cx="8" cy="8" r="0.8" fill="#111827" />
    </svg>
  ),
};

const EditorBrandIcon: React.FC<EditorBrandIconProps> = ({
  id,
  size = 16,
}) => <>{EDITOR_BRAND_ICONS[id]?.(size) ?? null}</>;

export default EditorBrandIcon;
