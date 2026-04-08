import { theme as antdTheme } from "antd";
import {
  legacyLogicalPropertiesTransformer,
  type Transformer,
} from "@ant-design/cssinjs";
import type { ThemeConfig } from "antd";

const DEVFLEET_ANTD_TOKEN: ThemeConfig["token"] = {
  colorPrimary: "#0d9488",
  borderRadius: 8,
  fontFamily:
    "'Plus Jakarta Sans', -apple-system, BlinkMacSystemFont, 'Microsoft YaHei', sans-serif",
  colorBgContainer: "#ffffff",
  colorBgElevated: "#ffffff",
  colorBgLayout: "#f5f7fa",
  colorBorder: "#dfe2e8",
  colorBorderSecondary: "#eceff4",
};

export const DEVFLEET_ANTD_COMPAT_TRANSFORMERS: Transformer[] = [
  legacyLogicalPropertiesTransformer,
];

// 统一 Ant Design 的主题 token，避免运行时和构建时静态导出的样式漂移。
export function getDevFleetAntdThemeConfig(isDark: boolean): ThemeConfig {
  return {
    algorithm: isDark ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
    token: {
      ...DEVFLEET_ANTD_TOKEN,
      colorBgContainer: isDark ? "#161a24" : "#ffffff",
      colorBgElevated: isDark ? "#1c2130" : "#ffffff",
      colorBgLayout: isDark ? "#0e1118" : "#f5f7fa",
      colorBorder: isDark ? "#262c3a" : "#dfe2e8",
      colorBorderSecondary: isDark ? "#1a1f2c" : "#eceff4",
    },
  };
}

export const DEVFLEET_LIGHT_ANTD_THEME = getDevFleetAntdThemeConfig(false);
export const DEVFLEET_DARK_ANTD_THEME = getDevFleetAntdThemeConfig(true);
