<div align="center">
  <img src="src/assets/jm-brand.png" width="96" height="96" alt="JM Client 图标" />
  <h1>JM Client</h1>
  <p><strong>禁漫天堂桌面客户端</strong></p>
  <p>发现 · 分类 · 排行 · 周刊 · 收藏 · 阅读 · 下载</p>
</div>

JM Client 基于 **Tauri 2 + Vue 3 + TypeScript + Rust + SQLite**，UI 与技术架构沿用 `pika_client`，数据源替换为 JM（18comic）App API。

## 功能

| 模块 | 说明 |
| --- | --- |
| 发现 | `/promote` 推荐分区、日/周/月排行榜、最新更新、本周必看 |
| 分类 | `/categories` 分类列表 + `/categories/filter` 分类/时间/排序筛选 |
| 排行榜 | 日榜 `mv_t` / 周榜 `mv_w` / 月榜 `mv_m`，支持分类筛选 |
| 每周必看 | `/week` 期刊列表 + `/week/filter` 日漫/其他/韩漫 |
| 搜索 | 全站 / 作品 / 作者 / 标签 / 角色（`main_tag=0..4`），时间与排序筛选，支持 `+标签` AND、`-标签` 排除 |
| 详情 | 标签、作品、角色、相关推荐、收藏、点赞、章节列表、批量下载 |
| 评论 | `/forum` 读取，登录后可发评论/回复，支持剧透遮罩 |
| 收藏 | 收藏夹分组、收藏时间/更新时间排序 |
| 观看历史 | `/watch_list`（需登录），结合本地阅读进度 |
| 阅读器 | 滚动/翻页、宽度、进度记忆、下一章、离线阅读；自动还原 JM 分块打乱的 WebP |
| 下载 | JPEG（默认）/ PNG / WebP，并发/间隔可调，暂停/恢复/取消，失败重试 |
| 本地书库 | 已下载漫画、章节、图片数、占用空间、磁盘清理、记录导入导出 |
| 设置 | API 线路与动态域名、图床/图源、代理、封面缓存、下载参数 |
| 版本更新 | 设置页底部检查正式版本、应用内下载与进度、SHA-256 校验、退出并安装；支持 Windows 与 Mac M 系列 |

游客可浏览、搜索、阅读和下载；收藏、评论、点赞、签到、观看历史需要 JM 账号。

## 下载与安装

在 [最新发布版本](https://github.com/mkktop/jm-client/releases/latest) 下载对应系统的安装包：

- Windows 10 / 11 x64：推荐 `*-setup.exe`，也可选择 `.msi`。
- Mac M 系列（Apple Silicon，macOS 11+）：下载 `*_aarch64.dmg`，打开后将 JM Client 拖入 Applications。Mac 版采用本地签名，未进行 Apple 公证；首次打开若被系统拦截，可在“系统设置 → 隐私与安全性”中允许打开。

已安装客户端可在设置页最下方检查更新。下载通过校验后，Windows 点击“退出并安装”；Mac 点击“退出并打开安装包”，再拖入 Applications 替换旧版。请先等待漫画下载完成，账号、书库和设置会保留。

## 技术栈

- 前端：Vue 3、Vue Router、Pinia、TypeScript、Vite、自定义 CSS
- 后端：Tauri 2、Rust、reqwest、sqlx/SQLite、tokio、image
- 图片：`jmimg://` 自定义协议（封面/头像缓存、章节图解码重排、离线图片）
- JM 协议：`/setting` 启动引导、token + AES-256-ECB 响应解密、AVS 会话、动态域名/图床

## 本地开发

环境要求：

- Node.js 22.12+
- Rust 1.90+（Windows 使用 MSVC 工具链）
- Microsoft C++ Build Tools / Windows SDK
- Microsoft Edge WebView2 Runtime

```bash
npm install
npm run tauri dev
```

前端开发服务固定使用 `http://127.0.0.1:1421`。

仅调试前端样式：

```bash
npm run dev
```

浏览器预览不包含 Tauri IPC，登录、分类、下载和数据库操作需要桌面运行环境。

常用命令：

```bash
npm run build                    # TypeScript 检查 + 前端构建
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run test:ui                  # Playwright 界面测试
npm run tauri build              # 构建安装包
```

应用图标采用“猫咪看漫画”设计，源图为 `src/assets/jm-desktop.png`。更换源图后运行 `npm run icons`，即可统一生成桌面与移动端图标，并同步应用内标识和网页 favicon。

## 使用说明

1. 首次启动可以直接点“以游客身份浏览”。
2. 在“偏好设置 → 网络连接”填写代理，例如 `http://127.0.0.1:7890` 或 `socks5://127.0.0.1:7891`。JM 通常需要代理才能访问。
3. 在“接口与图源”里选择 API 线路；切换线路后如已登录，需要重新登录。
4. 登录后可使用收藏、评论、点赞、签到和观看历史。
5. 在章节列表点击下载；下载管理页支持暂停、恢复、取消。

## 发布客户端更新

发布前同步修改 `package.json`（及 lockfile）、`src-tauri/Cargo.toml`（及 lockfile）与 `src-tauri/tauri.conf.json` 的版本号，并更新 `.github/release-notes.md`。推送对应标签（例如 `v0.2.0`），或在 Actions 中手动运行“Release desktop installers”并填写标签。

发布任务分别在 Windows x64 与 Mac ARM64 上执行原生测试和构建。Mac 安装包还会挂载检查、验证 arm64 架构与本地签名，并执行实际启动检查。两个平台均通过后，发布任务核对安装包与标签的源码、版本、格式和 SHA-256，生成校验清单，并将完整的 EXE / MSI / DMG 一起发布。

设置页底部的“软件更新”与 `pika_client` 使用相同的流程：从 GitHub Releases 检查最新正式版本，沿用已保存的代理设置，应用内下载对应架构的安装包并显示进度。下载完成和启动安装前均校验 SHA-256；没有适配当前系统且可校验的安装包时保留手动下载入口。草稿和预发布不会提示升级，没有正式发布时显示“暂未发布正式版本”。

## 数据目录

Windows 默认应用数据目录：

```text
%APPDATA%/com.jm.client/
├── jm.db          # 漫画、章节、下载记录、设置和 AVS 会话
└── downloads/     # 默认下载目录，可在设置中修改
```

下载图片按漫画名/章节名保存，文件名以页码编号。JM 原图是分块打乱的 WebP，下载时会解码重排并重新编码为用户选择的格式。

## 验证与测试

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run test:ui
```

Rust 单元测试覆盖 token、AES 解密、分块数量和纵向重排；`live_tests::live_jm_smoke` 是手动网络冒烟测试：

```bash
JM_PROXY=http://127.0.0.1:7897 cargo test --manifest-path src-tauri/Cargo.toml --lib live_jm_smoke -- --ignored --nocapture
```

## 免责声明

本工具仅作学习、研究、交流使用。使用本工具的用户应自行承担风险，作者不对使用本工具导致的任何损失、法律纠纷或其他后果负责。
