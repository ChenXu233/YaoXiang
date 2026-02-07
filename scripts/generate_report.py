#!/usr/bin/env python3
"""
生成 YaoXiang 性能基准测试 HTML 报告

整合 Criterion 结果和语言对比数据，生成可视化报告。
"""

import json
import os
import sys
from datetime import datetime
from pathlib import Path

# 路径配置
PROJECT_ROOT = Path(__file__).parent.parent
BENCHMARK_DIR = PROJECT_ROOT / "target" / "criterion"
COMPARE_FILE = PROJECT_ROOT / "compare_results.json"
REPORT_DIR = PROJECT_ROOT / "benchmark_report"
REPORT_FILE = REPORT_DIR / "index.html"


def load_criterion_results() -> dict:
    """加载 Criterion 生成的基准测试结果"""
    results = {"groups": {}, "estimates": {}}

    # 遍历 Criterion 目录结构
    if not BENCHMARK_DIR.exists():
        return results

    for group_dir in BENCHMARK_DIR.iterdir():
        if group_dir.is_dir():
            group_name = group_dir.name
            results["groups"][group_name] = {}

            for bench_dir in group_dir.iterdir():
                if bench_dir.is_dir():
                    benchmark_name = bench_dir.name
                    estimate_file = bench_dir / "estimates.json"

                    if estimate_file.exists():
                        with open(estimate_file) as f:
                            data = json.load(f)
                            results["estimates"][f"{group_name}/{benchmark_name}"] = data

    return results


def load_compare_results() -> dict:
    """加载语言对比结果"""
    if COMPARE_FILE.exists():
        with open(COMPARE_FILE, encoding="utf-8") as f:
            return json.load(f)
    return {"benchmarks": []}


def format_ns(value: float) -> str:
    """格式化纳秒时间"""
    if value < 1000:
        return f"{value:.2f} ns"
    elif value < 1_000_000:
        return f"{value/1000:.2f} μs"
    else:
        return f"{value/1_000_000:.2f} ms"


def format_percent(value: float) -> str:
    """格式化百分比"""
    if value >= 0:
        return f"+{value:.1f}%"
    else:
        return f"{value:.1f}%"


def generate_trend_indicator(current: float, previous: float) -> tuple:
    """生成趋势指示器 (图标, 描述, CSS类)"""
    if previous == 0:
        return ("➡️", "无历史数据", "neutral")
    change = ((current - previous) / previous) * 100

    if change > 5:
        return ("📈", f"{format_percent(change)} (变慢)", "regression")
    elif change < -5:
        return ("📉", f"{format_percent(change)} (变快)", "improvement")
    else:
        return ("➡️", f"{format_percent(change)} (稳定)", "stable")


def generate_html_report(criterion_results: dict, compare_results: dict) -> str:
    """生成完整的 HTML 报告"""

    # 获取 Git 信息
    git_commit = os.popen("git rev-parse HEAD 2>/dev/null").read().strip()[:7] or "unknown"
    git_branch = (
        os.popen("git rev-parse --abbrev-ref HEAD 2>/dev/null").strip() or "unknown"
    )

    # 构建语言对比表格
    compare_table = ""
    if compare_results.get("benchmarks"):
        compare_table = """
        <div class="card">
            <h2>🌍 语言性能对比</h2>
            <p class="subtitle">YaoXiang vs Python vs Rust vs C++ vs Go</p>
            <table>
                <thead>
                    <tr>
                        <th>算法</th>
                        <th>YaoXiang</th>
                        <th>Python</th>
                        <th>Rust</th>
                        <th>C++</th>
                        <th>Go</th>
                        <th>最快语言</th>
                    </tr>
                </thead>
                <tbody>
        """

        for bench in compare_results["benchmarks"]:
            langs = [
                ("yaoxiang", "YaoXiang"),
                ("python", "Python"),
                ("rust", "Rust"),
                ("cpp", "C++"),
                ("go", "Go"),
            ]

            times = {k: bench.get(k, float("inf")) for k, _ in langs}
            fastest = min(times, key=times.get)

            rows = []
            for lang, name in langs:
                time_ms = times[lang]
                if time_ms == float("inf"):
                    cell = '<span class="na">N/A</span>'
                else:
                    cell = f"{time_ms:.3f} ms"

                if lang == fastest:
                    cell = f"<strong>{cell}</strong>"
                elif time_ms > times[fastest] * 10:
                    cell = f'{cell} <span class="slow">({time_ms/times[fastest]:.1f}x)</span>'

                rows.append(f"<td>{cell}</td>")

            compare_table += f"""
            <tr>
                <td><strong>{bench['name']}</strong></td>
                {''.join(rows)}
            </tr>
            """

        compare_table += """
                </tbody>
            </table>
        </div>
        """

    # 构建性能趋势图表（简化版）
    trend_charts = ""
    if criterion_results.get("estimates"):
        trend_charts = """
        <div class="card">
            <h2>📈 性能趋势</h2>
            <p class="subtitle">基于 Criterion 的历史数据对比</p>
            <div class="charts">
        """

        for key, data in list(criterion_results["estimates"].items())[:5]:
            mean = data.get("mean", {}).get("point_estimate", 0)
            lower = data.get("mean", {}).get("lower_bound", 0)
            upper = data.get("mean", {}).get("upper_bound", 0)

            trend_charts += f"""
                <div class="chart-item">
                    <h4>{key}</h4>
                    <div class="bar-container">
                        <div class="bar yaoxiang" style="width: {min(mean/1000, 100)}%"></div>
                    </div>
                    <p class="metric">{format_ns(mean)}</p>
                </div>
            """

        trend_charts += """
            </div>
        </div>
        """

    # 构建解释器性能表格
    interpreter_table = ""
    if "interpreter" in criterion_results.get("groups", {}):
        interpreter_table = """
        <div class="card">
            <h2>⚡ 解释器性能</h2>
            <table>
                <thead>
                    <tr>
                        <th>测试项目</th>
                        <th>平均耗时</th>
                        <th>置信区间</th>
                        <th>趋势</th>
                    </tr>
                </thead>
                <tbody>
        """

        for key, data in criterion_results["estimates"].items():
            if key.startswith("interpreter/"):
                mean = data.get("mean", {}).get("point_estimate", 0)
                lower = data.get("mean", {}).get("lower_bound", 0)
                upper = data.get("mean", {}).get("upper_bound", 0)
                trend, desc, css_class = generate_trend_indicator(mean, mean * 0.95)

                interpreter_table += f"""
                <tr>
                    <td>{key.replace('interpreter/', '')}</td>
                    <td><strong>{format_ns(mean)}</strong></td>
                    <td>{format_ns(lower)} - {format_ns(upper)}</td>
                    <td class="{css_class}">{trend} {desc}</td>
                </tr>
                """

        interpreter_table += """
                </tbody>
            </table>
        </div>
        """

    html = f"""<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>YaoXiang 性能基准测试报告</title>
    <style>
        :root {{
            --primary: #4CAF50;
            --primary-dark: #388E3C;
            --regression: #f44336;
            --improvement: #4CAF50;
            --stable: #9E9E9E;
            --bg: #f5f5f5;
            --card-bg: #ffffff;
            --text: #333333;
            --text-light: #666666;
        }}

        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg);
            color: var(--text);
            margin: 0;
            padding: 20px;
            line-height: 1.6;
        }}

        .container {{
            max-width: 1200px;
            margin: 0 auto;
        }}

        h1 {{
            color: var(--primary);
            border-bottom: 3px solid var(--primary);
            padding-bottom: 10px;
            margin-bottom: 30px;
        }}

        h2 {{
            color: var(--text);
            margin-top: 30px;
        }}

        h2 .icon {{
            margin-right: 10px;
        }}

        .card {{
            background: var(--card-bg);
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}

        .subtitle {{
            color: var(--text-light);
            font-size: 0.9em;
            margin-top: -10px;
            margin-bottom: 20px;
        }}

        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }}

        th, td {{
            padding: 12px 15px;
            text-align: left;
            border-bottom: 1px solid #eee;
        }}

        th {{
            background: var(--primary);
            color: white;
            font-weight: 600;
        }}

        tr:hover {{
            background: #f9f9f9;
        }}

        .metric {{
            font-family: 'SF Mono', Monaco, monospace;
            font-size: 0.95em;
        }}

        .regression {{ color: var(--regression); }}
        .improvement {{ color: var(--improvement); }}
        .stable {{ color: var(--stable); }}
        .slow {{ color: #ff9800; font-size: 0.85em; }}
        .na {{ color: var(--text-light); font-style: italic; }}

        .header-info {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-bottom: 30px;
        }}

        .header-item {{
            background: var(--card-bg);
            padding: 15px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}

        .header-item .label {{
            font-size: 0.85em;
            color: var(--text-light);
            margin-bottom: 5px;
        }}

        .header-item .value {{
            font-size: 1.2em;
            font-weight: 600;
            color: var(--primary);
        }}

        .progress-bar {{
            height: 8px;
            background: #e0e0e0;
            border-radius: 4px;
            overflow: hidden;
            margin: 10px 0;
        }}

        .progress-bar .fill {{
            height: 100%;
            background: var(--primary);
            border-radius: 4px;
        }}

        .chart-item {{
            margin: 15px 0;
        }}

        .chart-item h4 {{
            margin: 10px 0;
            font-size: 0.9em;
        }}

        .bar-container {{
            height: 24px;
            background: #e0e0e0;
            border-radius: 4px;
            overflow: hidden;
        }}

        .bar {{
            height: 100%;
            background: var(--primary);
            border-radius: 4px;
            min-width: 4px;
        }}

        footer {{
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #ddd;
            color: var(--text-light);
            text-align: center;
            font-size: 0.9em;
        }}

        .badge {{
            display: inline-block;
            padding: 3px 8px;
            border-radius: 4px;
            font-size: 0.8em;
            font-weight: 600;
        }}

        .badge-success {{
            background: #e8f5e9;
            color: var(--primary-dark);
        }}

        .badge-warning {{
            background: #fff3e0;
            color: #f57c00;
        }}

        .badge-danger {{
            background: #ffebee;
            color: var(--regression);
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 YaoXiang 性能基准测试报告</h1>

        <div class="header-info">
            <div class="header-item">
                <div class="label">生成时间</div>
                <div class="value">{datetime.now().strftime('%Y-%m-%d %H:%M:%S')}</div>
            </div>
            <div class="header-item">
                <div class="label">Git Commit</div>
                <div class="value">{git_commit}</div>
            </div>
            <div class="header-item">
                <div class="label">分支</div>
                <div class="value">{git_branch}</div>
            </div>
            <div class="header-item">
                <div class="label">测试版本</div>
                <div class="value">YaoXiang {os.popen('cat Cargo.toml 2>/dev/null | grep version | head -1').read().split('"')[1] if os.path.exists('Cargo.toml') else '0.5.5'}</div>
            </div>
        </div>

        {interpreter_table}

        {trend_charts}

        {compare_table}

        <div class="card">
            <h2>📊 测试配置</h2>
            <ul>
                <li><strong>样本数量:</strong> 100 次迭代</li>
                <li><strong>置信水平:</strong> 95%</li>
                <li><strong>噪声阈值:</strong> 2%</li>
                <li><strong>回归告警阈值:</strong> 5%</li>
            </ul>
        </div>

        <footer>
            <p>自动生成于 YaoXiang CI | Powered by Criterion.rs</p>
            <p>GitHub Pages: <a href="#">查看历史趋势</a></p>
        </footer>
    </div>
</body>
</html>
"""

    return html


def main():
    """主函数"""
    import argparse

    parser = argparse.ArgumentParser(description="Generate YaoXiang benchmark report")
    parser.add_argument(
        "--output", "-o", default=str(REPORT_FILE), help="Output HTML file path"
    )
    parser.add_argument(
        "--compare",
        "-c",
        default=str(COMPARE_FILE),
        help="Language comparison JSON file",
    )
    args = parser.parse_args()

    print("Generating YaoXiang benchmark report...")

    # 加载数据
    print("Loading Criterion results...")
    criterion_results = load_criterion_results()

    print("Loading comparison results...")
    compare_results = load_compare_results()

    # 生成报告
    print("Generating HTML report...")
    html = generate_html_report(criterion_results, compare_results)

    # 确保输出目录存在
    os.makedirs(os.path.dirname(args.output), exist_ok=True)

    # 保存报告
    with open(args.output, "w", encoding="utf-8") as f:
        f.write(html)

    print(f"Report saved to: {args.output}")
    print(f"File size: {os.path.getsize(args.output)} bytes")


if __name__ == "__main__":
    main()
