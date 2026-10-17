#!/usr/bin/env python3
"""核对 think/ 档案重构的守恒性：基线提交与当前目录的节点数、边数应相等。

用法：
    python3 scripts/check_think_conservation.py --repo ../../data/profile

口径（与原 index.md 的手工命令一致）：
- 节点：基线取 fe651c1 的 think/ 与 work/ 下全部 .ttl，数 ` a pf:` 行并排除 IssueGroup；
  当前取 think/*.ttl，同样数 ` a pf:` 行并排除 Matter（新结构的事情组节点不在基线口径内）。
- 边：基线取 fe651c1 的 think/ 与 work/ 下全部 .lino，数含四个态射的行；
  当前取 think/*.lino，口径相同（跨组边记在 index.lino）。

只读；相等退出 0，不等退出 1。
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

DEFAULT_BASELINE = "fe651c1"
NODE_RE = re.compile(r" a pf:")
BASELINE_NODE_EXCLUDE = "IssueGroup"
CURRENT_NODE_EXCLUDE = "Matter"
EDGE_RE = re.compile(r"--(?:evokes|responds_to|serves|excludes)-->")


def git(repo: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr.strip() or f"git {' '.join(args)} 失败")
    return proc.stdout


def repo_root(path: Path) -> Path:
    return Path(git(path, "rev-parse", "--show-toplevel").strip())


def baseline_files(repo: Path, baseline: str, suffix: str) -> list[str]:
    names = git(repo, "ls-tree", "-r", baseline, "--name-only", "think", "work")
    return [n for n in names.splitlines() if n.endswith(suffix)]


def baseline_text(repo: Path, baseline: str, suffix: str) -> list[str]:
    return [git(repo, "show", f"{baseline}:{p}") for p in baseline_files(repo, baseline, suffix)]


def current_text(repo: Path, suffix: str) -> list[str]:
    return [p.read_text(encoding="utf-8") for p in sorted((repo / "think").glob(f"*{suffix}"))]


def count_nodes(texts: list[str], exclude: str) -> int:
    return sum(
        1
        for text in texts
        for line in text.splitlines()
        if NODE_RE.search(line) and exclude not in line
    )


def count_edges(texts: list[str]) -> int:
    return sum(
        1
        for text in texts
        for line in text.splitlines()
        if EDGE_RE.search(line)
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="核对 think/ 档案守恒（节点与边数）")
    parser.add_argument("--repo", default=".", help="profile 仓库路径（默认当前目录）")
    parser.add_argument("--baseline", default=DEFAULT_BASELINE,
                        help=f"基线提交（默认 {DEFAULT_BASELINE}）")
    args = parser.parse_args()

    repo = repo_root(Path(args.repo).resolve())
    try:
        base_nodes = count_nodes(baseline_text(repo, args.baseline, ".ttl"),
                                 BASELINE_NODE_EXCLUDE)
        cur_nodes = count_nodes(current_text(repo, ".ttl"), CURRENT_NODE_EXCLUDE)
        base_edges = count_edges(baseline_text(repo, args.baseline, ".lino"))
        cur_edges = count_edges(current_text(repo, ".lino"))
    except (RuntimeError, FileNotFoundError) as e:
        print(f"核对失败：{e}", file=sys.stderr)
        return 1

    ok = base_nodes == cur_nodes and base_edges == cur_edges
    print(f"think 守恒核对（基线 {args.baseline}）")
    print(f"  节点  基线 {base_nodes}  当前 {cur_nodes}  {'✓' if base_nodes == cur_nodes else '✗'}")
    print(f"  边    基线 {base_edges}  当前 {cur_edges}  {'✓' if base_edges == cur_edges else '✗'}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
