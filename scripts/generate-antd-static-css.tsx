import fs from "node:fs";
import path from "node:path";

// 这份静态样式需要和打包产物里的 Ant Design hash 完全一致，
// 所以必须在 production 模式下加载 antd/cssinjs 相关模块。
process.env.NODE_ENV = "production";

const OUTPUT_PATH = path.resolve(
  process.cwd(),
  "src/generated/antd-static.css",
);

type ReactRuntime = typeof import("react");
type AntdRuntime = Pick<
  typeof import("antd"),
  | "App"
  | "Button"
  | "ConfigProvider"
  | "Drawer"
  | "Input"
  | "Result"
  | "Select"
  | "Spin"
  | "Switch"
  | "Tag"
  | "Tooltip"
  | "Typography"
>;

function StaticAntdStyleFixture(
  React: ReactRuntime,
  antd: AntdRuntime,
) {
  const {
    Button,
    Drawer,
    Input,
    Result,
    Select,
    Spin,
    Switch,
    Tag,
    Tooltip,
    Typography,
  } = antd;

  return React.createElement(
    React.Fragment,
    null,
    React.createElement(
      "div",
      { style: { padding: 16 } },
      React.createElement(
        "div",
        { style: { display: "flex", gap: 12, marginBottom: 12 } },
        React.createElement(Button, { type: "primary" }, "Primary"),
        React.createElement(Button, null, "Default"),
        React.createElement(Button, { type: "primary", ghost: true }, "Ghost"),
        React.createElement(Button, { icon: React.createElement("span", null, "+") }, ""),
        React.createElement(Button, { danger: true }, "Danger"),
        React.createElement(Switch, { checked: true }),
        React.createElement(Tag, { color: "processing" }, "Tag"),
        React.createElement(Tag, { color: "blue" }, "Project"),
        React.createElement(Tag, { color: "green" }, "System"),
        React.createElement(Tag, { color: "default" }, "Empty"),
      ),
      React.createElement(
        "div",
        { style: { display: "grid", gap: 12 } },
        React.createElement(Input, {
          placeholder: "Search projects...",
          allowClear: true,
          prefix: React.createElement("span", null, "S"),
        }),
        React.createElement(Input.Search, {
          placeholder: "Search node version...",
          allowClear: true,
        }),
        React.createElement(Select, {
          style: { width: 220 },
          defaultValue: "dev",
          options: [
            { label: "dev", value: "dev" },
            { label: "build", value: "build" },
          ],
        }),
        React.createElement(Select, {
          style: { width: 220 },
          placeholder: "Select version",
          allowClear: true,
          options: [
            { label: "Node 22.17.1", value: "22.17.1" },
            { label: "Node 20.18.0", value: "20.18.0" },
          ],
        }),
        React.createElement(
          Typography.Paragraph,
          null,
          "/Users/example/project",
        ),
        React.createElement(
          Typography.Text,
          {
            copyable: true,
            ellipsis: { tooltip: "/Users/example/project" },
          },
          "/Users/example/project",
        ),
        React.createElement(
          Typography.Paragraph,
          {
            editable: {
              onChange: () => undefined,
            },
            ellipsis: { rows: 1, tooltip: true },
          },
          "Editable project note",
        ),
        React.createElement(Spin, { spinning: true }),
        React.createElement(
          Tooltip,
          { title: "Refresh projects" },
          React.createElement(Button, {
            icon: React.createElement("span", null, "R"),
          }),
        ),
      ),
      React.createElement(
        Drawer,
        {
          open: false,
          forceRender: true,
          getContainer: false,
          title: "Node Version Manager",
          width: 420,
        },
        React.createElement(Input, {
          placeholder: "Search versions...",
          allowClear: true,
        }),
      ),
      React.createElement(Result, {
        status: "info",
        title: "Empty State",
        subTitle: "Static style extraction keeps packaged builds stable.",
      }),
    ),
  );
}

function renderThemeSnapshot(
  React: ReactRuntime,
  antd: AntdRuntime,
  locale: Awaited<typeof import("antd/locale/zh_CN")>["default"],
  themeConfig: Awaited<typeof import("../src/theme/antdTheme")>["DEVFLEET_LIGHT_ANTD_THEME"],
) {
  const { App: AntdApp, ConfigProvider } = antd;

  return React.createElement(
    ConfigProvider,
    { locale, theme: themeConfig },
    React.createElement(
      AntdApp,
      null,
      StaticAntdStyleFixture(React, antd),
    ),
  );
}

async function collectStyles() {
  const React = await import("react");
  const { renderToString } = await import("react-dom/server");
  const antd: AntdRuntime = await import("antd");
  const { createCache, extractStyle, StyleProvider } = await import(
    "@ant-design/cssinjs"
  );
  const { default: enUS } = await import("antd/locale/en_US");
  const { default: zhCN } = await import("antd/locale/zh_CN");
  const {
    DEVFLEET_ANTD_COMPAT_TRANSFORMERS,
    DEVFLEET_DARK_ANTD_THEME,
    DEVFLEET_LIGHT_ANTD_THEME,
  } = await import("../src/theme/antdTheme");

  const snapshots = [
    renderThemeSnapshot(
      React,
      antd,
      zhCN,
      DEVFLEET_LIGHT_ANTD_THEME,
    ),
    renderThemeSnapshot(
      React,
      antd,
      zhCN,
      DEVFLEET_DARK_ANTD_THEME,
    ),
    renderThemeSnapshot(
      React,
      antd,
      enUS,
      DEVFLEET_DARK_ANTD_THEME,
    ),
  ];

  const cssChunks = snapshots.map((snapshot) => {
    const cache = createCache();
    renderToString(
      React.createElement(
        StyleProvider,
        {
          cache,
          hashPriority: "high",
          transformers: DEVFLEET_ANTD_COMPAT_TRANSFORMERS,
        },
        snapshot,
      ),
    );
    return extractStyle(cache, {
      plain: true,
      types: ["style", "token", "cssVar"],
    });
  });

  return cssChunks.join("\n");
}

async function main() {
  const css = await collectStyles();
  const banner =
    "/* This file is auto-generated by scripts/generate-antd-static-css.tsx. */\n";

  fs.mkdirSync(path.dirname(OUTPUT_PATH), { recursive: true });
  fs.writeFileSync(OUTPUT_PATH, `${banner}${css}`);
  console.log(`[antd-css] wrote ${path.relative(process.cwd(), OUTPUT_PATH)}`);
}

void main();
