# health reflect

记一条反思。形状取自认知行为疗法（CBT）的思维记录，再加两样量潮自己的东西：角色归属与行为实验。

```text
qtfounder-lab health reflect --stdin [--json]
qtfounder-lab health reflect --list [--due] [--json]
qtfounder-lab health reflect --id <ref_id> [--json]
qtfounder-lab health reflect --observe <ref_id> --result <一句话> [--emotion 0-100] [--json]
```

四个入口互斥，一次只用一个。没有交互问卷：能填表，反思就会退化成填表，问题得在人机对话里问出来，工具只负责存与翻。

## 为什么是这个形状

CBT 的思维记录是五栏：情境、情绪、自动思维、支持与反对的证据、替代思维，最后复评情绪强度。这套结构之所以有效，不在栏位多，而在每一栏都能被事实顶住——「他会谈砸」是判断，「团队谈过三次只差一个点」是事实。

行为实验再加一层闭环：写下一个能被推翻的预测，定一个复看日，到点回填实际发生了什么。放手实验就是行为实验的创始人版本——「我不碰两周，会出事吗」。

角色归属沿用 `think-reflect` 的两问：`domain` 判这件事属于公司还是我这个人，`control` 判是真的只有我能做（结构性稀缺）还是我不放心（心理性抓取）。这两个字段不参与评分，只是让记录能被归类。

## 记一条

**输入**：整条 JSON 从标准输入读入，字段名与 `--json` 输出一致。

```bash
qtfounder-lab health reflect --stdin --json <<'JSON'
{
  "situation": "周三亲自去谈供应商",
  "thought": "只有我去谈才不会谈砸",
  "emotion": "焦虑",
  "intensity": 70,
  "distortions": ["personalization", "catastrophizing"],
  "evidence_for": ["上次对方报价高"],
  "evidence_against": ["团队谈过三次，只差一个点"],
  "balanced": "我去能多拿一点，但团队够用；我去是因为不放心",
  "domain": "company",
  "control": "grasp",
  "experiment": {"prediction": "不碰两周会出小错", "review_on": "2026-11-07"}
}
JSON
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `situation` | 是 | 情境，一句话，具体到能判断 |
| `thought` | 是 | 自动思维，用当时的原话，不改写成道理 |
| `emotion` | 否 | 情绪名，如「焦虑」「愧疚」 |
| `intensity` | 否 | 情绪强度 0-100，与 `emotion` 成对出现 |
| `distortions` | 否 | 认知扭曲，见下表；中英文都认，落库统一存中文 |
| `evidence_for` | 否 | 支持这个念头的事实 |
| `evidence_against` | 否 | 反对这个念头的事实 |
| `balanced` | 否 | 替代思维，一句能站住的话 |
| `domain` | 否 | `company` 或 `self` |
| `control` | 否 | `scarce`（只有我能做）或 `grasp`（不放心） |
| `experiment` | 否 | 行为实验：`prediction` 必填，`review_on` 写 `YYYY-MM-DD` |

认知扭曲词表，英文键与中文标签互为别名：

| 键 | 标签 | 创始人常见的样子 |
|----|------|-----------------|
| `all_or_nothing` | 全或无 | 公司不好就是我不行 |
| `catastrophizing` | 灾难化 | 交出去一定砸 |
| `mind_reading` | 读心 | 他们肯定觉得我不管事 |
| `should` | 应该式 | 我应该随时在线 |
| `personalization` | 个人化 | 出事就是我的责任 |
| `emotional_reasoning` | 情绪化推理 | 我感到失控，所以确实失控了 |
| `discounting_positive` | 贬低积极面 | 做成了是运气 |
| `overgeneralization` | 过度概括 | 这次没做好，我什么都做不好 |
| `labeling` | 贴标签 | 我就是个控制狂 |
| `mental_filter` | 心理过滤 | 只看见错的那一处 |

**`--json` 输出**：

```json
{
  "id": "ref_Z6ZR",
  "at": "2026-10-24T13:52:04Z",
  "missing": [],
  "next_question": "到复看日回填结果：实际发生的事，比任何推理都算数。",
  "recurring": []
}
```

`missing` 按填栏顺序列出还空的栏位，`next_question` 是照第一处空缺给出的下一问。人读输出是同样三行：`已记 <id>`、`还差：…`、`下一问：…`。这一问由 agent 在对话里问，用户答完再补记一条，或者当场口述让 agent 补——工具不追问。

`recurring` 只列出现 ≥2 次的认知扭曲，按次数降序。这是反思里唯一值得立规则的信号：同一个思维陷阱反复出现，才够格写进规则文件。

## 看总表

```bash
qtfounder-lab health reflect --list [--due] [--json]
```

`--json` 输出三段：

```json
{
  "due": [
    {"id": "ref_AC9B", "situation": "让团队自己谈", "review_on": "2026-10-20", "prediction": "两周内出小错"}
  ],
  "open": [
    {"id": "ref_YJ13", "situation": "又接了活", "missing": ["emotion", "evidence_against"]}
  ],
  "recurring": [{"label": "个人化", "count": 2}]
}
```

`due` 是过了复看日、还没回填结果的实验；`open` 是栏位没填完的记录；`--due` 时只出 `due`，人读输出也只剩到期那一段。日期按 UTC 算，与 `at` 同一口径。

人读时 `open` 最多列最近 3 条，其余折成一句 `另有 N 条未列出`——欠账全摊出来，列表就成了新的负担。`--json` 不受此限，永远给全量。

## 回填实验结果

```bash
qtfounder-lab health reflect --observe ref_AC9B --result "只错了一个报价点，当天改回来了" --emotion 30
```

`--result` 必填，写实际发生了什么，不写感受；`--emotion` 是回填时的情绪强度，与记录时的 `intensity` 对照着看。记录里没有行为实验时不能回填，报用法错。

```json
{
  "id": "ref_AC9B",
  "prediction": "两周内出小错",
  "result": "只错了一个报价点，当天改回来了",
  "emotion_after": 30
}
```

回填不改写原来的判断，也不自动得出结论。预测被推翻这件事，由人和 agent 在对话里说，工具只把两条摆在一起。

## 看一条

```bash
qtfounder-lab health reflect --id ref_Z6ZR [--json]
```

`--json` 是记录本身加一个 `missing` 字段；人读逐行摊开，末尾同样给 `还差：…`。

## 落点

`<数据目录>/reflect.json`，单文件 JSON。数据目录与 `product` 一致：开发构建落 `src/cli/data/`，发布构建落 `$XDG_DATA_HOME/qtfounder-lab`，`QTFOUNDER_LAB_DATA` 覆盖一切。反思与需求差分开存，两套记录不互相污染。

## 错误

| 情况 | 退出码 | 输出 |
|------|--------|------|
| 四个入口给了零个或多个 | 2 | `{"error":{"code":"USAGE","message":"四选一：--stdin、--list、--id、--observe"}}` |
| 缺 `situation` / `thought`（空文本同） | 2 | `{"error":{"code":"USAGE","message":"缺字段 thought（不能为空）"}}` |
| 给 `emotion` 不给 `intensity`，或反过来 | 2 | `{"error":{"code":"USAGE","message":"给了 emotion 就要给 intensity（0-100）"}}` |
| `intensity` 超过 100 | 2 | `{"error":{"code":"USAGE","message":"intensity 是 0-100 的强度"}}` |
| 认不出的认知扭曲 | 2 | `{"error":{"code":"USAGE","message":"不认识的认知扭曲「瞎猜」，可用：全或无／灾难化／…"}}` |
| `domain` / `control` 取值不认识 | 2 | `{"error":{"code":"USAGE","message":"domain 只接受 company／self，收到「xxx」"}}` |
| `review_on` 不是 `YYYY-MM-DD` | 2 | `{"error":{"code":"USAGE","message":"review_on 要写 YYYY-MM-DD，收到「11月7日」"}}` |
| `--due` 不配 `--list`，或 `--result` / `--emotion` 不配 `--observe` | 2 | `{"error":{"code":"USAGE","message":"--due 只与 --list 一起用"}}` |
| `--result` 为空 | 2 | `{"error":{"code":"USAGE","message":"--result 不能为空"}}` |
| 记录没有行为实验却要回填 | 2 | `{"error":{"code":"USAGE","message":"ref_XXXX 没有行为实验，无从回填"}}` |
| `--id` / `--observe` 指向的记录不存在 | 3 | `{"error":{"code":"NOT_FOUND","message":"找不到反思 ref_ZZZZ","hint":"用 --list 看有哪些记录"}}` |

退出码三档与数据落点约定见 [index.md](index.md)。
