# intent（名词入口）

看意图。只读，不产生任何写入。

```text
qtfounder work intent [--id <id>] [--json]
```

| 参数 | 必填 | 说明 |
|------|------|------|
| `--id <id>` | 否 | 给定时查一件事的详情，不给则列出全部 |
| `--json` | 否 | 输出机器可读 JSON |

## qtfounder work intent

列出全部意图，每个带澄清次数和重复出现的差。

**人读输出**：

```text
int_01J9  给这段代码写单元测试        3 次澄清
  候选：给了大段铺陈而非可运行代码   ×2
int_01JA  把日志里的观察站段落切出来  1 次澄清
```

**`--json` 输出**：

```json
{
  "intents": [
    {
      "id": "int_01J9",
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
| `clarifications` | 该意图下的澄清记录条数 |
| `candidates` | 重复 ≥2 次的差；无候选时为空数组 |
| `actual_key` | `actual` 归一化后的键，同一键的多条即同类差 |

## qtfounder work intent --id <id>

一件事的详情：它下面的全部澄清记录。

**`--json` 输出**：

```json
{
  "id": "int_01J9",
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

**错误**：

| 情况 | 退出码 | 输出 |
|------|--------|------|
| `--id` 不存在 | 3 | `{"error":{"code":"NOT_FOUND","message":"int_01J9 不存在","hint":"用 qtfounder work intent 查看"}}` |
| 无 `--id` 但传了未知参数 | 2 | `{"error":{"code":"USAGE","message":"未知参数 --foo"}}` |
