## JM Client v0.1.3

- 升级方式与 pika_client 对齐：设置页底部检查更新，应用内下载、显示进度，并在下载完成和安装前校验 SHA-256。
- Windows 支持“退出并安装”；Mac 支持“退出并打开安装包”，拖入 Applications 替换旧版。
- 切换页面时保留更新下载进度和已校验状态，漫画正在下载或排队时阻止退出安装。
- 新增 Mac M 系列（Apple Silicon / ARM64）版本，修复跨平台图片协议，覆盖封面、头像、在线阅读和离线图片。
- 保留猫咪图标和侧栏账号区域的窗口适配修复。

### 安装

- Windows 10 / 11 x64：推荐下载 `*-setup.exe`，也可使用 `.msi`。
- Mac M 系列：下载 `*_aarch64.dmg`，适用于 Apple Silicon 和 macOS 11+，不提供 Intel 版本。打开后将 JM Client 拖入 Applications；升级时替换旧版，账号、书库和设置会保留。

Mac 版本采用本地签名，未进行 Apple 公证。首次打开若被系统拦截，可在“系统设置 → 隐私与安全性”中允许打开。

`SHA256SUMS.txt` 提供安装包的 SHA-256 校验值。
