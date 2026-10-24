"""存量巡检汇总：读 patrol.py 的 JSONL，按层与时间出数，并抽检消失的命中。

用法（在创始人仓根跑）：

    python3 examples/corpus-patrol/report.py . --data examples/corpus-patrol/data/findings.jsonl

三档分开算，是因为归宿不同：

- 随文件消失：整篇重写、改名或删除，旧命中自动清零，不算有人改过
- 在留存文件里改掉：有人动手，其中一部分是专门为这个词发起的清理
- 今天还在：仍然背着的存量

抽检只对最后一类里的术语做：取最近一次改动该词的提交，按提交信息里有没有
「术语」「用词」「说法」「语用」「命名」分类。这是启发式，改写顺带带走的不写。
"""

import argparse
import json
import re
import subprocess
from collections import Counter, defaultdict
from pathlib import Path


def classify(row: dict) -> str:
    """把一条 finding 归成 real / suspect / line0 / falsepos。"""
    if row.get("severity") == "suspect":
        return "suspect"
    if row.get("line", 0) == 0:
        return "line0"
    if row["audit"] == "language" and row.get("verdict") != "real":
        return "falsepos"
    return "docs" if row["audit"] == "docs" else "real"


def load(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def key(row: dict) -> tuple:
    return (row["path"], row["kind"], row["advice"])


def word_of(advice: str) -> str | None:
    found = re.match(r"「([^」]+)」", advice)
    return found.group(1) if found else None


def trace(root: Path, path: str, advice: str) -> bool | None:
    """回溯一次改动，判断是不是专门清理。查不到返回 None。"""
    word = word_of(advice)
    if not word:
        return None
    target = root / path
    repo = target.parent
    while repo != root and not (repo / ".git").exists():
        repo = repo.parent
    relative = str(target.relative_to(repo))
    proc = subprocess.run(
        ["git", "-C", str(repo), "--no-pager", "log", "-S", word, "--format=%s", "--", relative],
        capture_output=True,
        text=True,
    )
    lines = [line for line in proc.stdout.splitlines() if line.strip()]
    if not lines:
        return None
    return any(mark in lines[0] for mark in ("术语", "用词", "说法", "语用", "命名"))


def main() -> None:
    parser = argparse.ArgumentParser(description="存量巡检汇总")
    parser.add_argument("root", nargs="?", default=".", help="仓根，用来回溯提交")
    parser.add_argument("--data", default="examples/corpus-patrol/data/findings.jsonl")
    parser.add_argument("--spot", type=int, default=40, help="抽检多少条消失的术语命中")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    rows = load(Path(args.data))
    head = [r for r in rows if r["repo"] == "HEAD" and "kind" in r]
    snaps = [r for r in rows if r["repo"] != "HEAD" and "kind" in r]

    today = Counter(classify(r) for r in head)
    print("== 今天")
    print(
        f"真问题 {today['real'] + today['docs']}，嫌疑 {today['suspect']}，"
        f"误报 {today['falsepos']}，目录级体裁 {today['line0']}"
    )
    kinds = Counter(r["kind"] for r in head if classify(r) in ("real", "docs"))
    print("  按规则：" + "，".join(f"{k} {n}" for k, n in kinds.most_common()))
    print()

    by_repo = defaultdict(list)
    for row in snaps:
        by_repo[row["repo"]].append(row)

    print("== 按时间：每个快照的真问题数")
    for repo, items in sorted(by_repo.items()):
        per_date = Counter()
        for row in items:
            if classify(row) in ("real", "docs"):
                per_date[row["date"]] += 1
        print(f"{repo or '.':28s} " + "  ".join(f"{d[5:]}:{n}" for d, n in sorted(per_date.items())))
    print()

    head_keys = Counter(key(r) for r in head if classify(r) in ("real", "docs"))
    head_paths = {r["path"] for r in head}
    seen = {}
    for row in snaps:
        if classify(row) in ("real", "docs"):
            seen.setdefault(key(row), row)

    gone = [k for k in seen if k not in head_keys]
    fixed = [k for k in gone if k[0] in head_paths]
    print("== 三档")
    print(f"历史出现过 {len(seen)} 处不同命中，今天还在 {len(seen) - len(gone)} 处。")
    print(f"消失 {len(gone)} 处：随文件消失 {len(gone) - len(fixed)}，在留存文件里改掉 {len(fixed)}。")
    terminology = [k for k in gone if k[1] == "banned"]
    print(f"其中术语命中消失 {len(terminology)} 处。")
    print()

    print("== 规则榜：历史 → 今天")
    for (kind, advice), n in Counter((r["kind"], r["advice"]) for r in seen.values()).most_common(12):
        print(f"{n:5d} → {head_keys.get((kind, advice), 0):4d}  [{kind}] {advice}")
    print()

    candidates = [k for k in fixed if k[1] == "banned"][: args.spot]
    targeted = incidental = 0
    for path, kind, advice in candidates:
        verdict = trace(root, path, advice)
        if verdict is True:
            targeted += 1
        elif verdict is False:
            incidental += 1
    print("== 抽检")
    print(
        f"取 {len(candidates)} 条消失的术语命中回溯提交："
        f"{targeted} 条由术语清理提交改掉，{incidental} 条随改写消失。"
    )


if __name__ == "__main__":
    main()
