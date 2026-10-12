#!/usr/bin/env python3
"""check-corpus-parity: 语料差分门禁（09-execution-wbs 9.7，P2 引入）

逐文件比对两份 corpus_probe JSONL 记录：
- 诊断列表（code + span.file + span.line 逐条）
- 退出码
- 运行时 stdout/stderr（按行排序后的多重集）

**无任何分组放宽**（07-equivalence-oracle §三层判据；消息文本不参与比对的
归一化已在探针记录侧完成，本脚本只做逐字节等值判定）。

用法:
    python scripts/ci/check-corpus-parity.py <baseline.jsonl> <current.jsonl>

差分含义（P4 统一 Driver 等重构的 C2 判据）：
- 差分为空 = 行为等价；
- 差分非空 = 行为变化——或是回归（修代码），或是**有意变更**（在同一 PR
  重新生成基线：cargo run --quiet --example corpus_probe > tests/baselines/corpus-parity.jsonl）。

退出码: 0 = 无差分; 1 = 有差分 / 输入缺陷。
"""
import json
import sys


def load(path):
    records = {}
    with open(path, encoding="utf-8") as f:
        for i, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError as e:
                raise SystemExit(f"{path}:{i}: JSONL 解析失败: {e}")
            file = rec.get("file")
            if not file:
                raise SystemExit(f"{path}:{i}: 记录缺 file 字段")
            if file in records:
                raise SystemExit(f"{path}:{i}: 重复记录 {file}")
            records[file] = rec
    return records


def field_diffs(file, base, cur):
    """字段级差分报告（file/expectation/check/run 及其子字段）。"""
    out = []
    for key in sorted(set(base) | set(cur)):
        if key == "file":
            continue
        bv, cv = base.get(key), cur.get(key)
        if bv != cv:
            b_text = json.dumps(bv, ensure_ascii=False, sort_keys=True)
            c_text = json.dumps(cv, ensure_ascii=False, sort_keys=True)
            out.append(
                f"~ {file}: [{key}] 变化\n    基线: {b_text[:400]}\n    当前: {c_text[:400]}"
            )
    return out


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        return 1
    baseline = load(sys.argv[1])
    current = load(sys.argv[2])

    diffs = []
    for file in sorted(set(baseline) | set(current)):
        if file not in baseline:
            diffs.append(f"+ {file}（新增语料记录——若属预期新增语料，重新生成基线）")
        elif file not in current:
            diffs.append(f"- {file}（语料记录消失——语料被删或 skip/发现逻辑变化）")
        else:
            diffs.extend(field_diffs(file, baseline[file], current[file]))

    print(f"check-corpus-parity: 基线 {len(baseline)} 条 / 当前 {len(current)} 条")
    if not diffs:
        print("差分: 空（行为等价）")
        return 0
    print(f"差分: {len(diffs)} 处（**无任何分组放宽**）\n")
    for d in diffs[:50]:
        print(d)
    if len(diffs) > 50:
        print(f"... 其余 {len(diffs) - 50} 处略")
    print(
        "\n处置：回归则修代码；有意变更则在同一 PR 重新生成基线"
        "（cargo run --quiet --example corpus_probe > tests/baselines/corpus-parity.jsonl）"
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
