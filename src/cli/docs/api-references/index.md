# CLI API 参考

`src/cli` 的命令行接口。这里说的 API 不是 HTTP —— CLI 就是这套能力的唯一入口，它的入参、出参和退出码就是契约：模型按同样的形状调用，人按同样的形状阅读。

两个入口，名词读、动词写：

| 入口 | 形态 | 用途 | 详细 |
|------|------|------|------|
| `qtfounder-lab code requirement` | 名词 | 看有哪些需求、哪些差重复了 | [requirement.md](requirement.md) |
| `qtfounder-lab code clarify` | 动词 | 澄清一次，产出一段能粘回对话的话 | [clarify.md](clarify.md) |

> 实验室版的命令是 `qtfounder-lab`，与正式版 `qtfounder`（`apps/qtfounder/src/cli`）区分。在 `src/cli/` 下用 `cargo run --` 等价于这条命令，例如 `cargo run -- code clarify`。

## 通用约定

**入口不混用**：名词只读（列、查），动词只写（记录、立规则）。不设「一条命令既查又改」。

**人读默认，`--json` 给机器**。`--json` 的输出是稳定契约，字段只增不删。

**输入两条路**：人用逐个参数或交互作答；模型用 `--stdin` 把整条 JSON 投进去，字段名与 `--json` 输出一致。

**错误一律 JSON，写 stderr**：

```json
{"error":{"code":"NOT_FOUND","message":"req_01J9 不存在","hint":"用 qtfounder-lab code requirement 查看"}}
```

**退出码三档**：

| 码 | 含义 |
|----|------|
| 0 | 成功（含幂等去重） |
| 2 | 用法错（参数缺失、互斥、类型不对） |
| 3 | 未找到（`--id` 指向的需求不存在） |

顶层用法错同一条路：缺子命令给 `{"error":{"code":"USAGE","message":"缺少子命令：requirement 或 clarify"}}`，未知参数给 `未知参数 --foo`，退出码都是 2。

**幂等**：写入用的去重键是 sha256(before + actual)。重复提交不新增记录，返回已有那条并把 `deduplicated` 置为 true，退出码仍为 0——模型重试是安全的。

**数据落点**：开发构建（`cargo run`、`cargo test`）落**代码包内的 `data/`**（`src/cli/data/`），开发者直接看得到；发布构建遵循 XDG，落 `$XDG_DATA_HOME/qtfounder-lab`（未设则 `~/.local/share/qtfounder-lab`）。`QTFOUNDER_LAB_DATA` 优先级最高，测试与临时切换用它。目录里两个文件：`clarify.json`（账本）、`rules.md`（规则）。

## 数据形状

两层，没有第三层：

```text
requirement（一件事）
└── clarification[]（这次哪里不对）
```

规则不在这两层里，它就是 `rules.md` 的一行。

| 对象 | 字段 | 说明 |
|------|------|------|
| requirement | `id` | 自动生成，形如 `req_01J9` |
| | `task` | 一句话，这件事要干什么 |
| | `at` | 创建时间，ISO 8601 |
| clarification | `id` | 自动生成，形如 `clr_01J9` |
| | `before` | 我原以为它知道 / 会做什么 |
| | `actual` | 它实际做了什么；也是归类键 |
| | `prompt` | 可直接粘回对话的一段话 |
| | `at` | 记录时间，ISO 8601 |

没有第五个字段。`actual` 归一化（小写、去标点、去空白）后同样的出现 ≥2 次，就是一条规则候选。

**落盘**：数据目录默认 `data/`，环境变量 `QTFOUNDER_LAB_DATA` 覆盖整个数据目录（测试、多套数据用）。记录在 `<数据目录>/clarify.json`（单文件 JSON），规则在同目录 `rules.md` 的一行；哪批重复差已被规则处理，作为元数据记在数据文件里，不新增领域字段。
