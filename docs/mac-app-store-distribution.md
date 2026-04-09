# devFleet 分发到 Mac App Store 指南

本文档说明如何把 `devFleet` 分发到 Mac App Store，并明确指出当前仓库还需要补哪些改造。

适用范围：

- 技术栈：`Tauri 2 + React + Rust`
- 目标分发渠道：`Mac App Store`
- 校对时间：`2026-04-07`

## 1. 先说结论

把 `devFleet` 发到 Mac App Store，不是“把现有的 `.dmg` 或本地 build 直接上传”这么简单，而是要走一条单独的商店分发链路：

1. 在 App Store Connect 先创建应用记录。
2. 在 Apple Developer 后台准备 `App ID`、证书、`Mac App Store Connect` provisioning profile。
3. 让 Tauri 以 `App Store` 模式构建带沙盒能力的 `.app`。
4. 再把 `.app` 打成已签名的 `.pkg`。
5. 上传到 App Store Connect，补元数据、隐私信息、出口合规信息并提交审核。

对 `devFleet` 这个项目来说，真正的难点不是上传命令，而是：

- `App Sandbox`
- 自动更新能力
- 任意目录访问
- 外部终端 / 外部编辑器 / Shell 命令执行

基于 Apple 的沙盒规则和审核规则，我推断 `devFleet` 不能把当前桌面版功能原样搬进 Mac App Store，至少需要做一套单独的 `App Store` 变体。

## 2. Mac App Store 和当前发布方式的区别

当前仓库更接近“站外桌面分发”：

- 通过 Tauri 正常构建桌面应用
- 支持 Updater
- 默认允许更多本地能力

Mac App Store 分发则要求：

- 必须启用 `App Sandbox`
- 产物重点是上传到 App Store Connect 的 `.pkg`
- 更新应通过 App Store，不应保留自更新逻辑
- 对文件系统、网络、外部进程、权限申请都要做最小化声明

这意味着 Mac App Store 版最好单独维护一套配置，不要和现有站外发布配置混在一起。

## 3. 当前仓库和上架相关的风险点

结合当前代码，至少有下面几个高风险项：

### 3.1 自更新能力需要对 App Store 版关闭

当前仓库启用了：

- `src-tauri/tauri.conf.json` 里的 updater 配置
- `src-tauri/src/lib.rs` 里的 `tauri_plugin_updater`
- `src/components/UpdateChecker.tsx` 里的更新检查与重启逻辑
- `src-tauri/capabilities/default.json` 里的 `updater:default`

Apple 的审核规则要求应用保持自包含，不要下载、安装或执行会改变应用功能的代码。对 App Store 版来说，最稳妥的做法是：

1. 单独做一套 `App Store` build 配置。
2. 关闭 updater 插件和前端更新入口。
3. 把“检查更新”改成“请通过 App Store 更新”。

### 3.2 任意项目目录访问需要按沙盒方式重做

当前应用允许用户选择任意项目目录，并把路径持久化：

- `src/lib/tauri.ts`
- `src-tauri/src/config.rs`

Apple 的 App Sandbox 会限制应用访问文件系统；如果要长期访问用户手动选择的目录，通常需要使用 `security-scoped bookmarks` 一类的机制来持久化授权。

基于官方沙盒文档，我推断目前“保存普通绝对路径，之后反复读取”的方式，不足以直接满足 Mac App Store 版本的长期目录访问需求。

### 3.3 外部终端 / 编辑器 / Shell 执行是审核高风险区

当前应用包含：

- 在外部终端运行脚本：`src-tauri/src/commands.rs`
- 打开第三方编辑器：`src-tauri/src/commands.rs`、`src-tauri/src/detector.rs`
- 本地 Node 管理和命令执行：`src-tauri/src/node_manager.rs`

基于 Apple 的 App Sandbox 和审核规则，我推断这些能力是 `devFleet` 上架 Mac App Store 的最大风险点之一：

- 需要验证沙盒下是否还能工作
- 即使技术上能工作，也可能被要求说明用途
- 若实现方式涉及超出容器的任意命令执行，审核风险会明显上升

如果目标是“尽快上架”，更现实的方案通常是：

- 为 Mac App Store 做一个能力收敛版
- 或至少把高风险能力做成可关闭的 `app-store` 变体

## 4. 上架前的准备清单

### 4.1 Apple 账号和工具

需要准备：

- 已加入 Apple Developer Program 的账号
- 可登录 App Store Connect 的权限
- 一台 macOS 机器
- 最新稳定版 Xcode 和命令行工具

### 4.2 App Store Connect 应用记录

在上传 build 之前，先在 App Store Connect 里创建应用记录。至少要填写：

- 平台：`macOS`
- App 名称
- 主语言
- Bundle ID
- SKU

这里的 `Bundle ID` 必须和 Tauri 配置里的 `identifier` 一致。当前仓库是：

```json
"identifier": "com.niesugar.devfleet"
```

### 4.3 证书和 Provisioning Profile

你需要准备 App Store 分发所需的签名材料，包括：

- 显式 `App ID`
- 分发证书
- `Mac App Store Connect` provisioning profile

Apple 官方文档明确说明，macOS 上传 App Store Connect 时应使用 `Mac App Store Connect` 类型的 profile。

## 5. 推荐的仓库改造方式

建议不要直接改现有 `src-tauri/tauri.conf.json`，而是单独新增一套 App Store 配置。

推荐文件布局：

```text
src-tauri/
├── tauri.conf.json
├── tauri.appstore.conf.json
├── Entitlements.plist
├── Info.plist
└── profiles/
    └── devfleet-mas.provisionprofile
```

这样可以把：

- 站外 DMG / 开发版
- Mac App Store 版

分成两条构建链，互不影响。

## 6. App Store 版需要补的关键文件

### 6.1 `Entitlements.plist`

Mac App Store 版必须启用 `App Sandbox`。

最低示例：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>com.apple.security.app-sandbox</key>
  <true/>

  <key>com.apple.application-identifier</key>
  <string>TEAM_ID.com.niesugar.devfleet</string>

  <key>com.apple.developer.team-identifier</key>
  <string>TEAM_ID</string>

  <key>com.apple.security.network.client</key>
  <true/>

  <key>com.apple.security.files.user-selected.read-write</key>
  <true/>
</dict>
</plist>
```

说明：

- `TEAM_ID` 要替换成你的 Apple Team ID。
- `network.client` 适合 `devFleet` 访问远端版本列表、检查联网能力。
- `user-selected.read-write` 适合“用户通过对话框选择项目目录”的场景。
- 仍然应遵循最小权限原则，只保留你实际需要的 entitlement。

### 6.2 `Info.plist`

按 Tauri 官方文档，建议在 `src-tauri` 下补 `Info.plist`。最常见的是出口合规字段：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>ITSAppUsesNonExemptEncryption</key>
  <false/>
</dict>
</plist>
```

如果应用使用了需要单独申报的加密能力，这里不能直接填 `false`，要按真实情况处理。

### 6.3 `tauri.appstore.conf.json`

建议新建独立配置文件，至少处理这些点：

- `bundle.category`
- `bundle.macOS.entitlements`
- `bundle.macOS.files.embedded.provisionprofile`
- `bundle.macOS.bundleVersion`

示例：

```json
{
  "bundle": {
    "category": "Utility",
    "macOS": {
      "entitlements": "./Entitlements.plist",
      "bundleVersion": "100",
      "files": {
        "embedded.provisionprofile": "./profiles/devfleet-mas.provisionprofile"
      }
    }
  }
}
```

说明：

- `category` 是 App Store 展示所需字段。
- `bundleVersion` 建议使用单调递增的内部 build 编号。
- provisioning profile 文件会被打进 `.app` 包内。

## 7. 构建和上传流程

### 7.1 本地构建 App Store 版 `.app`

在 macOS 机器上执行：

```bash
pnpm install
pnpm build
pnpm tauri build --bundles app --target universal-apple-darwin --config src-tauri/tauri.appstore.conf.json
```

如果你只支持 Apple Silicon，也可以只构建 `aarch64-apple-darwin`。

### 7.2 用 `productbuild` 生成可上传的 `.pkg`

Tauri 官方给出的 macOS App Store 路径是：先产出 `.app`，再用 `productbuild` 签成 `.pkg`。

示例：

```bash
APP_NAME="devFleet"

xcrun productbuild \
  --sign "<Mac Installer Distribution 证书名>" \
  --component "src-tauri/target/universal-apple-darwin/release/bundle/macos/${APP_NAME}.app" \
  /Applications \
  "${APP_NAME}.pkg"
```

这里的证书要使用可用于 App Store 的安装器分发证书。

### 7.3 上传到 App Store Connect

可以用 `altool` 上传：

```bash
xcrun altool \
  --upload-app \
  --type macos \
  --file "devFleet.pkg" \
  --apiKey "$APPLE_API_KEY_ID" \
  --apiIssuer "$APPLE_API_ISSUER"
```

如果你用 App Store Connect API Key，Tauri 文档建议把密钥文件保存为：

```text
AuthKey_<APPLE_API_KEY_ID>.p8
```

常见放置位置包括：

- `./private_keys`
- `~/private_keys`
- `~/.private_keys`
- `~/.appstoreconnect/private_keys`

## 8. 上传后在 App Store Connect 要补什么

上传 build 只是第一步，后续还要在 App Store Connect 里完成这些信息：

- 应用描述
- 关键词
- 分类
- 隐私政策 URL
- 截图
- App 隐私问卷
- 出口合规信息
- 价格与可用地区

在一切填写完成后，再把某个 build 关联到该版本并提交审核。

## 9. `devFleet` 上架前的建议改造顺序

建议按下面的顺序推进，风险最低：

1. 先做独立的 `tauri.appstore.conf.json`、`Entitlements.plist`、`Info.plist`。
2. 给 App Store 版禁用 updater。
3. 给前端加一个 `app-store` 构建开关，隐藏更新入口。
4. 验证在沙盒下，用户选择项目目录后，项目列表能否在重启后继续访问。
5. 重新设计或裁剪“外部终端运行脚本 / 外部编辑器打开 / Node 管理”这些高风险能力。
6. 本机完成一次完整签名、打包、上传验证。
7. 再补 App Store Connect 元数据并提审。

## 10. 这个项目最可能卡住审核的地方

如果从“审核风险”角度排序，我认为当前仓库最值得优先处理的是：

1. 自更新能力
2. 超出沙盒边界的文件访问
3. 外部命令执行和第三方程序拉起
4. 项目路径长期持久化但没有 security-scoped bookmark
5. App Store 版本与站外版本没有明确分离

## 11. 建议的 App Store 版策略

如果你只是想“先上架”，建议采用下面这个策略：

- 保留一个站外完整版：继续 DMG / 直装 / 自更新
- 再做一个 Mac App Store 版：能力收敛、通过审核优先

对 `devFleet` 来说，这通常比“强行把现有完整版原样送审”更现实。

## 12. 官方资料

以下资料是编写本文时参考的主要官方来源：

- Apple：Add a new app  
  https://developer.apple.com/help/app-store-connect/create-an-app-record/add-a-new-app
- Apple：Create an App Store Connect provisioning profile  
  https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile
- Apple：App Sandbox  
  https://developer.apple.com/documentation/security/app-sandbox
- Apple：Configuring the macOS App Sandbox  
  https://developer.apple.com/documentation/xcode/configuring-the-macos-app-sandbox/
- Apple：App Review Guidelines  
  https://developer.apple.com/appstore/resources/approval/guidelines.html
- Apple：Security-scoped bookmark options  
  https://developer.apple.com/documentation/Foundation/NSURL/BookmarkCreationOptions/securityScopeAllowOnlyReadAccess
- Tauri：App Store  
  https://v2.tauri.app/zh-cn/distribute/app-store/
- Tauri：macOS Application Bundle  
  https://v2.tauri.app/distribute/macos-application-bundle/

## 13. 一句话行动建议

如果下一步真的要推进上架，不要先改 CI；先做 `App Store` 变体，把：

- updater 关掉
- entitlement 补齐
- 文件访问改成沙盒可接受的方式
- 高风险功能做开关或裁剪

等这些跑通以后，再补自动化打包和上传。
