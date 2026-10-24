# requirement（名词入口）

看需求。只读，不产生任何写入。

```text
qtfounder-lab code requirement [--id <id>] [--json]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `--id <id>` | 否 | 给定时查一件事的详情，不给则列出全部 |
| `--json` | 否 | 输出机器可读 JSON |

## qtfounder-lab code requirement

列出全部需求，每个带澄清次数和重复出现的差。

**人读输出**（两空格分列；候选只列重复 ≥2 次的差；没有需求时输出 `（暂无需求，先跑 qtfounder-lab code clarify）`）：

```text
req_01J9  给这段代码写单元测试  3 次澄清
  候选：给了大段铺陈而非可运行代码  ×2
req_01JA  把日志里的观察站段落切出来  1 次澄清
```

**`--json` 输出**：

```json
{
  "requirements": [
    {
      "id": "req_01J9",
      "task": "给这段代码写单元测试",
      "at": "2026-10-18T09:00:00Z",
      "clarifications": 3,
      "candidates": [
        {
          "actual_key": "给了大段铺陈而非可运行代码",
          "count": 2,
          "clarification_ids": ["clr_01J9", "clr_01JA"]
        }
      ]
    }
  ]
}
```

| 字段 | 说明 |
|------|------|
| `clarifications` | 该需求下的澄清记录条数 |
| `candidates` | 重复 ≥2 次的差；无候选时为空数组 |
| `actual_key` | `actual` 归一化后的键，同一键的多条即同类差 |

## qtfounder-lab code requirement --id <id>

一件事的详情：它下面的全部澄清记录。

**`--json` 输出**：

```json
{
  "id": "req_01J9",
  "task": "给这段代码写单元测试",
  "at": "2026-10-18T09:00:00Z",
  "clarifications": [
    {
      "id": "clr_01J9",
      "at": "2026-10-18T09:12:03Z",
      "before": "我以为它知道我要能直接跑的代码",
      "actual": "给了五段解释加伪代码",
      "prompt": "刚才你的回答偏在铺陈展开，我需要的是能直接跑的代码……"
    }
  ]
}
```

**人读输出**（`原以为/实际/提示词` 三行展开每条澄清）：

```text
req_01J9  给这段代码写单元测试  2026-10-18T09:00:00Z
  clr_01J9  2026-10-18T09:12:03Z
    原以为：我以为它知道我要能直接跑的代码
    实际：给了五段解释加伪代码
    提示词：我原以为「我以为它知道我要能直接跑的代码」。
它实际做了「给了五段解释加伪代码」。
请按我原以为的重做一遍，把这两条当约束。
```

**错误**：

| 情况 | 退出码 | 输出 |
|------|--------|------|
| `--id` 不存在 | 3 | `{"error":{"code":"NOT_FOUND","message":"req_01J9 不存在","hint":"用 qtfounder-lab code requirement 查看"}}` |
| 传了未知参数 | 2 | `{"error":{"code":"USAGE","message":"未知参数 --foo"}}` |
| `--id` 缺值（clap 提示，取首行） | 2 | `{"error":{"code":"USAGE","message":"error: a value is required for '--id <id>' but none was supplied"}}` |
