# PageFix 📄✨

> 把倾斜拍摄的纸张，一键矫正成“正对视角”的桌面扫描小工具。  
> Built with **Tauri + Vue 3 + Rust + OpenCV**.

## 🌟 项目亮点

- 🎥 **实时摄像头预览**（Linux /dev/video\*）
- 📸 **截屏即暂停**，方便精调当前帧
- 🔷 **多边形编辑**：拖拽顶点、在线上加点、右键删点
- 🧭 **透视矫正**：将多边形区域变换到正视图
- 🧪 **后处理模式**：无 / 锐化（默认锐化）
- 🔄 **结果旋转**：左旋/右旋 90°
- 🔍 **双窗口缩放拖拽**：便于精细对齐

## 🧱 技术栈

- 前端：Vue 3 + TypeScript + Vite
- 桌面壳：Tauri v2
- 后端：Rust
- 视觉算法：OpenCV（Rust crate）
- 摄像头采集：rscam（V4L2）

## 🚀 快速开始

### 1) 安装依赖（Debian/Ubuntu）

```bash
sudo apt update
sudo apt install -y \
  libopencv-dev pkg-config clang libclang-dev cmake build-essential \
  libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
  libxdo-dev libssl-dev patchelf
```

如果访问 `/dev/video*` 被拒绝：

```bash
sudo usermod -aG video $USER
```

执行后重新登录系统再试。

### 2) 安装前端依赖

本项目的 Tauri 配置默认使用 `bun` 驱动前端构建。

```bash
bun install
```

### 3) 启动开发

```bash
bun run tauri dev
```

## 🕹️ 使用流程

1. 选择摄像头、分辨率和 FPS，点击“开启预览”
2. 点击“截屏”冻结当前帧
3. 在上方窗口拖拽/增删顶点，包住纸张区域
4. 点击“矫正”，在下方窗口查看结果
5. 需要时使用“左旋转90° / 右旋转90°”

## 🧠 算法简述

- 📍 轮廓检测：多通道阈值 + 边缘 + 轮廓筛选，提取文档四边形
- 🪄 透视变换：根据顶点计算单应矩阵（Perspective Transform）
- 🧼 文本后处理：去噪、对比度增强、阈值化，提升文档可读性

## 📁 目录结构

```text
.
├─ src/
│  ├─ App.vue
│  └─ features/camera/
│     ├─ constants.ts
│     ├─ types.ts
│     ├─ utils.ts
│     └─ composables/
│        ├─ usePanZoom.ts
│        └─ useCameraScanner.ts
└─ src-tauri/
   └─ src/
      ├─ main.rs
      ├─ lib.rs        # Tauri command + 状态管理
      └─ vision.rs     # OpenCV 检测/矫正/后处理
```

## 🛠️ 常用命令

```bash
# 前端开发
bun run dev

# 前端构建
bun run build

# Tauri 开发
bun run tauri dev

# Rust 格式化 / 检查
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

## ⚠️ 注意事项

- 目前摄像头采集逻辑主要针对 **Linux**。
- 若系统源异常（签名/镜像问题），先修复 `apt` 源再安装依赖。
- 如果不用 `bun`，请把 `src-tauri/tauri.conf.json` 里的 `beforeDevCommand` / `beforeBuildCommand` 改成 `npm run dev` / `npm run build`。

## 📌 项目目标

PageFix 的目标很简单：  
**用最少交互，把手机/摄像头拍到的纸张快速变成清晰、规整、可读的扫描图。** ✅
