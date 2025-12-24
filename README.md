# rightcode-floatingball

一个基于 `iced` 的桌面悬浮球，用于展示 RightCode 订阅的剩余额度/水位。

## 功能

- 悬浮置顶、无边框、可拖动
- 右键立即刷新
- 鼠标滚轮切换订阅
- 右下角拖拽调整悬浮球大小
- 设置页支持配置 `Authorization token / Cookie(cf_clearance) / User-Agent / 刷新间隔`
- 系统托盘菜单：刷新 / 设置 / 退出
- 设置页支持开机自启动（Windows/macOS）

## 本地运行

```bash
cargo run
```

## 配置说明

点击悬浮球右上角齿轮进入设置页，配置文件路径会在设置页顶部显示。

注意：不要将真实的 `Authorization` / `cf_clearance` 等敏感信息提交到仓库。

## 开发

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

## 自动化发布（cargo-dist）

本仓库使用 `cargo-dist` 生成 GitHub Actions 发布流程，目前仅构建 Windows/macOS 客户端。

- 打 tag 并 push：`git tag vX.Y.Z && git push --tags`
- GitHub Actions 会构建并将产物上传到对应的 GitHub Release

参考文档：`https://axodotdev.github.io/cargo-dist/`
