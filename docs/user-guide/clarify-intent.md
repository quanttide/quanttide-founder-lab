# 澄清与意图：怎么用

`qtfounder-lab work clarify` 记录一次「AI 不对劲」，`qtfounder-lab work intent` 看这些记录攒出了什么。这条命令不分阶段：开始前、卡住时、结束后敲的是同一句，工具按「有没有可挂的事」决定新开一件还是续记。

> `>` 后面是你要敲进去的答案，回车提交。答得笼统（比如「它就是不行」）会被追问一次「举它刚才那句话或那个动作」——概括是情绪，不是信息。

## 开始前：把期待说清楚

新起一件事，连着回答两问：

```bash
qtfounder-lab work clarify --task "给这段代码写单元测试"
```

```text
你原本以为它知道 / 会做什么？
> 它知道项目用 pytest，fixture 在 conftest，会直接给出可运行的测试
它实际做了什么？
> 给了五段解释加伪代码，没有一行能跑

我原以为「它知道项目用 pytest，fixture 在 conftest，会直接给出可运行的测试」。
它实际做了「给了五段解释加伪代码，没有一行能跑」。
请按我原以为的重做一遍，把这两条当约束。

已记入 int_1SAZ / clr_FYEY
```

中间那三行就是拿去粘回对话的话。你不用自己组织语言，改两行也能用——它只是把你刚才的两问排成了 AI 能直接照改的形状。

## 卡住时：接着记，不用再给任务

同一件事还在做，直接再敲一次，它挂到最近一件事上，不问任务：

```bash
qtfounder-lab work clarify
```

```text
你原本以为它知道 / 会做什么？
> 它知道我要三行以内的结论，先给结果
它实际做了什么？
> 先讲了两大段背景才给答案

我原以为「它知道我要三行以内的结论，先给结果」。
它实际做了「先讲了两大段背景才给答案」。
请按我原以为的重做一遍，把这两条当约束。

已记入 int_1SAZ / clr_VTAV
```

记完就走：把中间那段贴回对话，看它这次改不改。

## 结束后：看攒出了什么

```bash
qtfounder-lab work intent
```

```text
int_1SAZ  给这段代码写单元测试  3 次澄清
  候选：先讲了两大段背景才给答案  ×2
```

每件事一行：编号、任务、澄清几次。下面缩进的「候选」是重复出现 ≥2 次的差——同一个毛病犯第二次，就该浮出来了。

看某件事的全部记录：

```bash
qtfounder-lab work intent --id int_1SAZ
```

```text
int_1SAZ  给这段代码写单元测试  2026-10-18T08:12:42Z
  clr_FYEY  2026-10-18T08:12:42Z
    原以为：它知道项目用 pytest，fixture 在 conftest，会直接给出可运行的测试
    实际：给了五段解释加伪代码，没有一行能跑
    提示词：我原以为「它知道项目用 pytest，fixture 在 conftest，会直接给出可运行的测试」。
它实际做了「给了五段解释加伪代码，没有一行能跑」。
请按我原以为的重做一遍，把这两条当约束。
  clr_VTAV  2026-10-18T08:12:42Z
    原以为：它知道我要三行以内的结论，先给结果
    实际：先讲了两大段背景才给答案
    提示词：……
```

加 `--json` 换成机器可读输出（脚本、模型用）：

```bash
qtfounder-lab work intent --json
qtfounder-lab work intent --id int_1SAZ --json
```

## 规则怎么长出来

一个毛病犯两次 → 成为候选 → 你点头挑一条写成规则，工具把它追加进 `rules.md`：

```bash
qtfounder-lab work clarify --rule "先结论后细节，除非我明确要展开"
```

```text
已追加规则到 data/rules.md
- 先结论后细节，除非我明确要展开
已处理 2 条重复差：clr_VTAV, clr_9S2V
```

规则文本永远是你自己写，工具不替你生成——它只负责把候选顶到你面前。落库后：

- `rules.md` 一行一条规则，从上到下就是这套协作方式的演进史；
- 这批重复差标为已处理，下次 `--rule` 的 `source` 只会给你新的候选。

所以完整的一条线是：**记两问 → 同类差重复 ≥2 次浮成候选 → 你写一条规则 → 下次开工前把 `rules.md` 给 AI 看**。

## 模型（脚本）怎么用

一次把整条记录给全，字段名和 `--json` 输出一致：

```bash
echo '{"task":"把日志里的观察站段落切出来","before":"它知道只切段落不加解释","actual":"整段照搬没切分","prompt":"按段落切开重发"}' \
  | qtfounder-lab work clarify --stdin --json
```

```json
{
  "id": "clr_12WN",
  "intent_id": "int_XNRM",
  "at": "2026-10-18T08:12:42Z",
  "deduplicated": false,
  "prompt": "按段落切开重发"
}
```

同一条再提交一次不会新增，`deduplicated` 变 `true`，退出码仍是 0——重试是安全的：

```json
{
  "id": "clr_12WN",
  "intent_id": "int_XNRM",
  "at": "2026-10-18T08:12:42Z",
  "deduplicated": true,
  "prompt": "按段落切开重发"
}
```

## 出错时看什么

错误一律是 JSON、写在 stderr，退出码三档：`0` 成功（含去重），`2` 用法错（参数缺失、互斥、类型不对），`3` 没找到（`--id` 指向的事不存在）。

```bash
$ qtfounder-lab work intent --id int_没有  # 退出码 3
{"error":{"code":"NOT_FOUND","message":"int_没有 不存在","hint":"用 qtfounder-lab work intent 查看"}}
```

常见用法错：`--stdin` 少了 `before`/`actual`/`prompt`、`--task` 和 `--rule` 同时给、`--rule` 文本为空。逐参数的完整契约见 [接口参考](../../src/cli/docs/api-references/index.md)。

## 记录存在哪

默认在当前目录 `data/` 下（`clarify.json` 存记录、`rules.md` 存规则）。要换位置，设环境变量 `QTFOUNDER_LAB_DATA` 指向一个目录，整个数据目录（记录 + 规则）都跟着走。
