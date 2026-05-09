# XutsFileBackup

一款基于 **Tauri v2 + Rust + React** 的 Windows 桌面文件镜像备份工具。将源目录单向镜像到目标目录，并提供文件索引、全文搜索与层级标签管理。

---

## 功能特性

### 镜像同步
- **单向镜像**：源目录 A → 目标目录 B，B 完全跟随 A
  - A 新增 / 修改的文件 → 自动复制到 B
  - A 删除的文件 → B 同步删除
  - A 中的目录结构 → 完整复制，孤立目录自动清理
- **三种触发方式**：
  - 手动触发（一次性同步）
  - 实时监控（文件系统事件驱动，秒级响应）
  - 定时触发（Cron 表达式，如 `0 2 * * *` 每天凌晨 2 点）
- **过滤规则**：Glob 路径排除（如 `node_modules/**`、`*.tmp`）

### 标签系统
- 多级层级标签树（自引用结构，支持无限嵌套）
- 对任意已索引文件 / 目录打标签，可附加备注
- 搜索时支持多标签 AND / OR 过滤，且自动向下展开子标签
- 标签仅存于应用数据库，不修改文件系统

### 搜索与浏览
- FTS5 全文检索（文件名 + 相对路径）
- 结合标签过滤，支持 AND（所有标签都满足）或 OR（任意标签满足）
- 文件浏览器支持目录树导航（面包屑路径栏）
- 已打标签的文件 / 目录在文件列表中显示对应颜色圆点

### 其他
- 系统托盘：关闭窗口最小化到托盘，左键恢复，右键菜单可退出
- 同步日志面板：每次同步的详细记录（扫描数、复制数、删除数、错误）
- React 错误边界：UI 崩溃时显示友好提示，支持重试

---

## 技术栈

| 层 | 技术 |
|----|------|
| 桌面框架 | [Tauri v2](https://tauri.app/) |
| 后端 | Rust + tokio 异步运行时 |
| 前端 | React 19 + TypeScript |
| UI 样式 | Tailwind CSS |
| 数据库 | SQLite（rusqlite + FTS5 全文索引） |
| 文件监控 | [notify](https://github.com/notify-rs/notify) crate |
| 定时任务 | [cron](https://crates.io/crates/cron) + chrono |
| 文件哈希 | xxHash-64（内容变更校验） |

---

## 目录结构

```
XutsFileBackup/
├── src/                        # React 前端
│   ├── App.tsx                 # 根组件，状态管理
│   ├── main.tsx                # 入口，挂载错误边界
│   ├── lib/
│   │   └── api.ts              # Tauri IPC 封装
│   └── components/
│       ├── Sidebar.tsx         # 任务列表 + 标签树
│       ├── SearchBar.tsx       # 搜索框 + 标签筛选条
│       ├── FileTable.tsx       # 文件浏览器（树 / 平铺双模式）
│       ├── FileTagEditor.tsx   # 文件标签管理弹窗
│       ├── TaskDialog.tsx      # 任务创建 / 编辑对话框
│       ├── TagDialog.tsx       # 标签创建 / 编辑对话框
│       ├── SyncLogPanel.tsx    # 同步日志面板
│       ├── Toasts.tsx          # 通知 Toast
│       └── ErrorBoundary.tsx   # React 错误边界
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── lib.rs              # 应用入口，托盘，窗口事件
│   │   ├── commands.rs         # Tauri Command 注册
│   │   ├── db/
│   │   │   ├── mod.rs          # DbState（Mutex<Connection>）
│   │   │   └── migrations.rs   # 数据库版本迁移
│   │   ├── sync/
│   │   │   ├── engine.rs       # 核心同步逻辑（diff + 执行）
│   │   │   ├── scanner.rs      # 目录扫描
│   │   │   ├── diff.rs         # 新增 / 修改 / 删除差量计算
│   │   │   └── executor.rs     # 文件复制 / 删除 / 哈希
│   │   ├── watcher.rs          # 文件系统实时监控（notify）
│   │   ├── scheduler.rs        # Cron 定时调度
│   │   ├── search.rs           # FTS5 + 标签 CTE 搜索
│   │   ├── tags.rs             # 标签 CRUD + 关联管理
│   │   ├── task_manager.rs     # 任务 CRUD
│   │   └── error.rs            # 统一错误类型
│   ├── icons/                  # 应用图标
│   ├── capabilities/           # Tauri 权限配置
│   └── tauri.conf.json         # Tauri 构建配置
├── DESIGN.md                   # 系统设计文档
├── package.json
└── README.md
```

---

## 数据库 Schema

```sql
-- 备份任务
tasks (id, name, source_path, target_path, trigger_type, cron_expr, filter_rules, enabled, created_at)

-- 文件索引（含目录，kind = 'file' | 'dir'）
file_records (id, task_id, relative_path, size, mtime, content_hash, status, last_synced_at, kind)

-- 标签（自引用层级树）
tags (id, name, color, parent_id, description, created_at)

-- 文件-标签多对多
file_tags (file_id, tag_id, tagged_at, note)

-- 同步日志
sync_logs (id, task_id, started_at, finished_at, files_scanned, files_copied, files_deleted, bytes_transferred, errors, status)

-- FTS5 全文索引
file_search_index (file_id UNINDEXED, filename, relative_path, tag_names)
```

---

## 开发环境要求

| 工具 | 版本 |
|------|------|
| [Rust](https://rustup.rs/) | 1.77+ |
| [Node.js](https://nodejs.org/) | 18+ |
| [Visual Studio C++ 构建工具](https://visualstudio.microsoft.com/visual-cpp-build-tools/) | 2019+ |
| Windows | 10 / 11 |

> macOS / Linux 未经测试，Tauri v2 理论上支持，但文件路径处理和图标需适配。

---

## 快速开始

```bash
# 1. 克隆仓库
git clone https://github.com/xu-tiansong/XutsFileBackup.git
cd XutsFileBackup

# 2. 安装前端依赖
npm install

# 3. 开发模式（热重载）
npm run tauri dev

# 4. 生产构建
npm run tauri build
# 输出：src-tauri/target/release/bundle/
#   nsis/XutsFileBackup_0.1.0_x64-setup.exe   ← NSIS 安装向导
#   msi/XutsFileBackup_0.1.0_x64_en-US.msi    ← Windows Installer
```

---

## 同步引擎工作流

```
Phase A  读取任务配置 + 从数据库加载当前文件索引快照
Phase B  遍历源目录，镜像所有子目录到目标目录，更新 DB 中 kind='dir' 记录
Phase C  扫描源目录文件列表（跳过目录）
Phase D  计算差量（新增 / 修改 / 删除），逐文件执行：
           新增 → 复制 + 写入 file_records + 写入 FTS5 索引
           修改 → 比对 xxHash，内容变更则复制，更新 file_records
           删除 → 删除目标文件，CASCADE 清除 file_tags，删除 FTS5 条目
Phase E  清理目标目录中已不存在于源目录的孤立子目录（深度优先）
Phase G  写入同步日志
```

---

## 标签搜索原理

标签过滤使用递归 CTE 向下展开整棵子树，再 JOIN file_tags：

```sql
-- 搜索"工作"标签时，自动命中"财务/合同"、"财务/报销"、"会议"等子标签下的所有文件
WITH RECURSIVE tag_tree(id, root_id) AS (
  SELECT id, id FROM tags WHERE id IN (?)
  UNION ALL
  SELECT t.id, tt.root_id FROM tags t
  JOIN tag_tree tt ON t.parent_id = tt.id
),
tag_match(file_id) AS (
  SELECT ft.file_id FROM file_tags ft
  JOIN tag_tree tt ON ft.tag_id = tt.id
  GROUP BY ft.file_id
  -- AND 模式：HAVING COUNT(DISTINCT tt.root_id) = <n>
)
SELECT fr.* FROM file_records fr
JOIN tag_match tm ON fr.id = tm.file_id
WHERE fr.status = 'active'
```

---

## 数据存储位置

应用数据库（`xuts.db`）存储于系统标准应用数据目录：

```
Windows: C:\Users\<用户名>\AppData\Roaming\com.xuts.filebackup\xuts.db
```

卸载应用不会自动删除此文件，标签和索引数据得以保留。

---

## License

MIT
