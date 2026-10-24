"""存量巡检采集：把历史快照按今天的契约重判，落到 JSONL。

用法（在创始人仓根跑）：

    python3 examples/quanttide-founder-lab/examples/corpus-patrol/patrol.py . --out examples/corpus-patrol/data/findings.jsonl

做了什么：找出根下所有 git 仓，每个仓按提交数均匀抽若干次，各自导出成一棵树
（路径保持根目录相对，契约里的 exclude 才对得上），跑 language 与 docs 两套审计，
把逐条结果与扫描篇数写进一份 JSONL。汇总交给 report.py。

为什么要抽样：`data/context` 有三千次提交，主仓也有一千八，全取太慢。
两端必取、中间等距，`--samples 0` 表示全取。

与 `qtfounder audit --patrol` 的分工：那条命令给总数与比例，够判断闸门值不值；
这个脚本留逐条明细，用来分层、分时间看，也能换契约重跑。
"""

import argparse
import json
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

MAIN = Path(__file__).resolve().parents[4]
DEFAULT_CONTRACTS = MAIN / "apps/qtfounder/src/cli/src"
HERE = Path(__file__).resolve().parent


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def discover(root: Path) -> list[Path]:
    """根自己，加上各层子模块（子模块的 .git 是文件）。"""
    repos = []
    for path in root.rglob(".git"):
        parent = path.parent
        if any(part in (".git", "target", "node_modules", "_build") for part in parent.parts):
            continue
        repos.append(parent)
    return sorted(set(repos))


def pick_commits(repo: Path, samples: int) -> list[tuple[str, str]]:
    """按提交数均匀抽样：两端必取，中间等距。samples=0 表示全取。"""
    log = git(repo, "log", "--reverse", "--format=%H %as")
    rows = [tuple(line.split()) for line in log.splitlines() if line.strip()]
    if not rows:
        return []
    if samples <= 0 or len(rows) <= samples:
        return rows
    if samples == 1:
        return rows[-1:]
    picked = []
    for slot in range(samples):
        row = rows[slot * (len(rows) - 1) // (samples - 1)]
        if not picked or picked[-1][0] != row[0]:
            picked.append(row)
    return picked


def export(repo: Path, sha: str, workspace: Path, root: Path) -> Path:
    """导出到 workspace/相对路径，让契约的路径匹配照常生效。"""
    dest = workspace / repo.relative_to(root)
    dest.mkdir(parents=True, exist_ok=True)
    tar = workspace / "snapshot.tar"
    git(repo, "archive", "--format=tar", "-o", str(tar), sha)
    with tarfile.open(tar) as archive:
        archive.extractall(dest, filter="data")
    tar.unlink()
    return dest


def scan(root: Path, contracts: Path) -> list[dict]:
    findings = []
    for name in ("language", "docs"):
        proc = subprocess.run(
            [
                "qtfounder",
                name,
                ".",
                "--rules",
                str(contracts / name / "default-contract.toml"),
                "--json",
            ],
            cwd=root,
            capture_output=True,
            text=True,
        )
        if not proc.stdout.strip():
            print(f"!! {name} 没有输出：{proc.stderr.strip()[:200]}", file=sys.stderr)
            continue
        payload = json.loads(proc.stdout)
        findings.append({"audit": name, "scanned": payload.get("scanned_files", 0)})
        for item in payload.get("findings", []):
            item["audit"] = name
            findings.append(item)
    return findings


def clear(directory: Path) -> None:
    for entry in directory.iterdir():
        if entry.is_dir():
            shutil.rmtree(entry)
        else:
            entry.unlink()


def main() -> None:
    parser = argparse.ArgumentParser(description="存量巡检采集")
    parser.add_argument("root", nargs="?", default=".", help="仓根，缺省当前目录")
    parser.add_argument("--out", default=str(HERE / "data/findings.jsonl"))
    parser.add_argument("--samples", type=int, default=12, help="每个仓抽多少个提交，0 表示全取")
    parser.add_argument("--contracts", default=str(DEFAULT_CONTRACTS), help="两份契约所在目录")
    args = parser.parse_args()

    root = Path(args.root).resolve()
    contracts = Path(args.contracts).resolve()
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    workspace = out.parent / "snapshot"

    rows = []

    # 今天：整根扫一遍，路径与快照同一口径。
    for item in scan(root, contracts):
        rows.append({"repo": "HEAD", "date": "today", "sha": "worktree", **item})

    repos = discover(root)
    print(f"找到 {len(repos)} 个仓", file=sys.stderr)
    for repo in repos:
        name = str(repo.relative_to(root)) or "."
        commits = pick_commits(repo, args.samples)
        print(f"{name}：{len(commits)} 次提交", file=sys.stderr)
        for index, (sha, date) in enumerate(commits, 1):
            if workspace.exists():
                shutil.rmtree(workspace)
            dest = export(repo, sha, workspace, root)
            for item in scan(workspace, contracts):
                rows.append({"repo": name, "date": date, "sha": sha[:8], **item})
            clear(dest)
            print(f"\r  {index}/{len(commits)} {date} {sha[:8]}", end="", file=sys.stderr)
        print(file=sys.stderr)
    if workspace.exists():
        shutil.rmtree(workspace)

    with out.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(f"写入 {out}，{len(rows)} 行", file=sys.stderr)


if __name__ == "__main__":
    main()
