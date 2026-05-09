# XutsFileBackup — 系统设计文档

> 最后更新：2026-05-09

---

## 项目概述

单向文件镜像备份 App。将源目录 A 的内容完整镜像同步到目标目录 B，并在 App 内部数据库中维护文件索引与标签系统，支持对所有已索引文件进行搜索和标签管理。

---

## 需求范围（已确认）

### 镜像模式
- **单向镜像**：A → B，B 完全跟随 A
  - A 中新增/修改的文件 → 复制到 B
  - A 中删除的文件 → B 中同步删除

### 触发方式
- 手动触发（一次性同步）
- 实时监控（文件系统事件驱动，秒级响应）
- 定时触发（Cron 表达式）

### 过滤规则
- 按路径 Glob 排除（如 `node_modules/`、`.git/`、`*.tmp`）
- 按文件大小上限（不同步超过 N MB 的单文件）

### 标签系统（已确认决策）
| 决策项 | 选择 |
|--------|------|
| 标签层级 | 支持多级层级（自引用树结构） |
| 文件删除时标签处理 | 硬删除，标签级联清除 |
| 标签存储位置 | 仅存于 App 数据库，不写入文件系统 |
| 搜索范围 | App 内所有已索引文件均可搜索，标签为可选过滤维度 |

---

## 系统架构

```
┌──────────────────────────────────────────────────┐
│                   UI Layer (Tauri)                │
│  TaskList │ TaskEditor │ FilesBrowser │ Settings  │
└────────────────┬─────────────────────────────────┘
                 │ IPC (Tauri Commands / Events)
┌────────────────▼─────────────────────────────────┐
│               Core Engine (Rust)                  │
│                                                   │
│  ┌─────────────┐  ┌──────────────┐  ┌─────────┐ │
│  │ FileWatcher  │  │  Scheduler   │  │  Queue  │ │
│  │ (notify-rs) │  │ (tokio-cron) │  │ (async) │ │
│  └──────┬──────┘  └──────┬───────┘  └────┬────┘ │
│         └────────────────┴───────────────┘       │
│                          │                        │
│              ┌───────────▼───────────┐            │
│              │    SyncEngine         │            │
│              │  · diff 计算          │            │
│              │  · xxHash 校验        │            │
│              │  · 并发复制 (rayon)   │            │
│              │  · 级联删除处理       │            │
│              └───────────┬───────────┘            │
│                          │                        │
│              ┌───────────▼───────────┐            │
│              │   Local File Adapter  │            │
│              └───────────────────────┘            │
│                                                   │
│  ┌──────────────────────────────────────────────┐ │
│  │    State DB (SQLite)                          │ │
│  │  tasks · file_records · tags · file_tags     │ │
│  │  sync_logs · FTS5 index                      │ │
│  └──────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────┘
```

---

## 数据库 Schema

```sql
-- 任务表
CREATE TABLE tasks (
  id            INTEGER PRIMARY KEY,
  name          TEXT    NOT NULL,
  source_path   TEXT    NOT NULL,
  target_path   TEXT    NOT NULL,
  trigger_type  TEXT    NOT NULL DEFAULT 'manual', -- manual | realtime | schedule
  cron_expr     TEXT,
  filter_rules  TEXT    NOT NULL DEFAULT '{}',     -- JSON: { exclude_globs[], max_size_mb }
  enabled       INTEGER NOT NULL DEFAULT 1,
  created_at    TEXT    NOT NULL
);

-- 文件索引（A/B 共享一条记录，以 relative_path 为唯一键）
CREATE TABLE file_records (
  id             INTEGER PRIMARY KEY,
  task_id        INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  relative_path  TEXT    NOT NULL,   -- 相对于 source_path，如 "docs/report.docx"
  size           INTEGER NOT NULL,
  mtime          TEXT    NOT NULL,
  content_hash   TEXT    NOT NULL,   -- xxHash64
  status         TEXT    NOT NULL DEFAULT 'active', -- active | deleted
  last_synced_at TEXT    NOT NULL,
  UNIQUE(task_id, relative_path)
);

-- 标签表（支持层级，自引用）
CREATE TABLE tags (
  id          INTEGER PRIMARY KEY,
  name        TEXT    NOT NULL UNIQUE,
  color       TEXT    NOT NULL DEFAULT '#6366f1', -- hex color
  parent_id   INTEGER REFERENCES tags(id) ON DELETE SET NULL,
  description TEXT,
  created_at  TEXT    NOT NULL
);

-- 文件-标签关联（多对多）
CREATE TABLE file_tags (
  file_id    INTEGER NOT NULL REFERENCES file_records(id) ON DELETE CASCADE,
  tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  tagged_at  TEXT    NOT NULL,
  note       TEXT,
  PRIMARY KEY (file_id, tag_id)
);

-- 同步日志
CREATE TABLE sync_logs (
  id                INTEGER PRIMARY KEY,
  task_id           INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
  started_at        TEXT    NOT NULL,
  finished_at       TEXT,
  files_scanned     INTEGER NOT NULL DEFAULT 0,
  files_copied      INTEGER NOT NULL DEFAULT 0,
  files_deleted     INTEGER NOT NULL DEFAULT 0,
  bytes_transferred INTEGER NOT NULL DEFAULT 0,
  errors            TEXT    NOT NULL DEFAULT '[]', -- JSON array
  status            TEXT    NOT NULL DEFAULT 'running' -- running | success | partial_error | failed
);

-- FTS5 全文检索虚拟表
CREATE VIRTUAL TABLE file_search_index USING fts5(
  file_id UNINDEXED,
  filename,        -- 从 relative_path 提取的文件名
  relative_path,
  tag_names        -- 空格分隔的所有关联标签名（含祖先标签）
);
```

---

## 标签层级设计

### 树结构示例
```
工作
├── 财务
│   ├── 合同
│   └── 报销
└── 会议
个人
├── 旅行
└── 照片
```

### 搜索向上匹配规则
- 给文件打"合同"标签
- 搜索"财务" → 命中所有打了"合同"或"报销"的文件（展开子孙）
- 搜索"工作" → 命中"财务"和"会议"下所有文件

### 递归标签查询 SQL
```sql
WITH RECURSIVE tag_tree AS (
  SELECT id FROM tags WHERE name = ?        -- 输入的标签名
  UNION ALL
  SELECT t.id FROM tags t
  JOIN tag_tree tt ON t.parent_id = tt.id
)
SELECT DISTINCT fr.*
FROM file_records fr
JOIN file_tags ft ON fr.id = ft.file_id
WHERE ft.tag_id IN (SELECT id FROM tag_tree)
  AND fr.status = 'active';
```

---

## 文件删除级联流程

```
镜像引擎检测到文件从 A 中消失
          │
          ▼
  删除 B 中对应文件
          │
          ▼
  DELETE file_tags WHERE file_id = ?    ← 级联（ON DELETE CASCADE）
          │
          ▼
  DELETE file_records WHERE id = ?
          │
          ▼
  从 FTS5 index 中移除对应条目
```

SQLite 外键 `ON DELETE CASCADE` 自动处理 file_tags 级联，代码层只需删除 file_records。

---

## Diff 算法

```
扫描阶段:
  源目录  → Map<relative_path, (mtime, size)>
  目标目录 → Map<relative_path, (mtime, size)>
  数据库  → Map<relative_path, (mtime, size, hash)>

对比阶段:
  新增: 在源中存在，数据库中不存在              → 复制 + 写入 file_records
  修改: mtime 或 size 与数据库记录不同
        → 计算 xxHash 确认内容变更              → 复制 + 更新 file_records
  删除: 数据库中 active 但源中不存在            → 删除目标文件 + 级联清理
  无变化: mtime+size 匹配，跳过（不计算 hash）

优化: 优先用 mtime+size 快速判断，hash 仅在不确定时计算
```

---

## 搜索设计

```
用户输入搜索词
       │
       ├─── 无标签过滤 ──► FTS5 搜索 filename + relative_path
       │
       └─── 有标签过滤 ──► CTE 递归展开标签树，得到所有子孙 tag id
                                 │
                                 ▼
                          JOIN file_tags 过滤文件集合
                                 │
                                 ▼
                          可选：再叠加 FTS5 文本匹配
```

标签过滤支持多标签 AND/OR 逻辑。

---

## UI 结构

```
┌─ 侧边栏 ────────────────────────────────────────────┐
│  [任务列表]                                          │
│   ▶ 工作文档备份  ● 实时同步                         │
│   ▶ 照片备份      ○ 已暂停                           │
│                                                      │
│  [标签树]                                            │
│   ▼ 工作                                             │
│     ▼ 财务                                           │
│       ● 合同  (8)                                    │
│       ● 报销  (3)                                    │
│     ● 会议    (12)                                   │
│   ▶ 个人                                             │
└──────────────────────────────────────────────────────┘

┌─ 主区域 ─────────────────────────────────────────────┐
│  🔍 [搜索框: 输入文件名或路径...] [+ 标签过滤 ▼]     │
│     标签筛选条: [× 财务] [× 合同]  逻辑: [AND ▼]    │
│                                                      │
│  文件名           路径          大小   修改时间  标签 │
│  report_2026.docx docs/finance/ 120KB 2026-05-01 合同│
│  lease_agreement  docs/         890KB 2026-03-15 合同│
│                                                      │
│  右键菜单: 打开文件 | 管理标签 | 在资源管理器中显示  │
└──────────────────────────────────────────────────────┘
```

### 页面列表
- **任务列表**：所有备份任务，状态、上次同步时间、操作按钮
- **任务编辑**：源/目标路径、触发方式、过滤规则配置
- **文件浏览器**：跨任务搜索、标签过滤、右键打标签
- **标签管理**：标签树 CRUD、颜色设置、文件数统计
- **同步日志**：每次同步的详细记录、错误信息
- **设置**：全局配置、通知偏好

---

## 技术栈

| 模块 | 技术 | 说明 |
|------|------|------|
| 桌面框架 | Tauri v2 | Rust 后端 + Web 前端，Windows 原生权限 |
| 前端框架 | React + TypeScript | |
| UI 组件库 | shadcn/ui + Tailwind CSS | |
| 后端语言 | Rust | 高性能文件 IO，无 GC 停顿 |
| 异步运行时 | tokio | 高并发异步 IO |
| 并发复制 | rayon | 多核数据并行 |
| 文件监控 | notify crate | 跨平台 FS 事件，Windows IOCP |
| 哈希算法 | xxhash-rust | 极速非密码学哈希，文件内容比对 |
| 定时任务 | tokio-cron-scheduler | Cron 表达式调度 |
| 数据库 | SQLite (rusqlite + FTS5) | 嵌入式，含全文检索和外键级联 |
| 序列化 | serde + JSON | Tauri IPC 数据交换 |
| 打包 | Tauri bundler | 生成 .exe / .msi |

---

## 开发计划

| Phase | 内容 | 状态 |
|-------|------|------|
| **1** | 项目脚手架 + 完整 SQLite schema（FTS5、外键级联） | ✅ 完成 |
| **2** | 单向镜像引擎（diff 算法 + 文件复制/删除 + 级联清理） | ✅ 完成 |
| **3** | 文件监控（realtime）+ Cron 调度 + 手动触发 | ✅ 完成 |
| **4** | 标签 CRUD + 层级标签树 + file_tags 关联 API + 任务 CRUD | ✅ 完成 |
| **5** | 搜索引擎（FTS5 + 递归标签 CTE 组合查询） | ✅ 完成 |
| **6** | 文件浏览器 UI + 标签树侧边栏 + 搜索界面 + 文件标签编辑 | ✅ 完成 |
| **7** | 任务管理 UI + 实时进度推送 + 同步日志面板 | ✅ 完成 |
| **8** | 系统托盘 + 错误边界处理 + 打包发布 | 待开始 |

**总计约 29 个工作日**

---

## 已知风险

| 风险 | 应对策略 |
|------|---------|
| Windows 文件锁定（被其他进程占用） | 捕获错误，加入重试队列，日志记录 |
| 大目录初始扫描性能 | mtime+size 快速比对，hash 按需计算，结果缓存到 SQLite |
| 符号链接/硬链接 | 第一版跳过符号链接，记录警告日志 |
| FTS5 索引与 file_records 不一致 | 每次增删改同步更新，启动时做一次全量校验 |
| 标签树深度过深影响查询性能 | CTE 递归查询，SQLite 默认支持 1000 层递归，实际够用 |
