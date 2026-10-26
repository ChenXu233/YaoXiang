#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
check-fanout.py —— 禁令三（该重构不补丁）门禁：同义重复 / 扇出 / 入口接线

规则本体：docs/src/dev/coding-rules.md 第一部分·禁令三（判据 A/B/C）
  判据 C 原文：「新增"第 N 个入口 / 表项"而非登记进声明式阶段表 → 违规」。
入口清单权威：RFC-039 §1.1（5 个手工接线入口的 11 处不一致）
任务来源：docs/src/dev/architecture/09-execution-wbs.md §P0 任务 0.2.2
引入阶段：P0（report-only，默认退出码恒 0）；P9 统一转硬（CI 改为 --strict）。

判据实现：
  C. 入口接线（主检查）   函数体内直接调用 ≥2 个不同阶段原语即判定为
                         「手工接线的入口」。阶段原语按**调用形态**标定，覆盖
                         面（标定依据见下文「C 判据的重定基」）：
                         - 12 个 Stage 变体的**臂调用形态**（self.<臂>(…)，臂名
                           由 Driver::dispatch 的穷尽 match 派生，单一源）；
                         - 其中 6 个变体另有可在 Driver 之外调用的**共享原语**：
                           Parsing / Typecheck / ProofExecution /
                           DeadCodeAnalysis / IrGeneration / Monomorphization；
                         - Codegen / Exec 两类执行层原语（无对应 Stage 变体）。
                         其余 6 个变体分两类，不能笼统说成「只有臂形态」：
                         (a) GlobalSlotAlloc / Linking 的阶段实现是 Driver **内私有**
                         （pub(in crate::driver)：src/driver/helpers.rs 的
                         allocate_global_slots、link_module_irs），Driver 之外无处可调；
                         (b) VendorConsistency / Discovery / Registry /
                         RoleClassification 的阶段实现是 Driver **之外**的共享原语
                         （src/frontend/module/orchestrator.rs 的
                         ensure_vendor_consistency / discover、discover_with_used /
                         build_registry、build_registry_from；src/frontend/module/
                         roles.rs 的 classify）。本表**只**收录这四类的臂形态，它们的
                         共享原语形态是**已知未覆盖（待裁决）**的盲区。
                         盲区可复算——把这四类原语（ensure_vendor_consistency(、
                         discover( / discover_with_used(、build_registry( /
                         build_registry_from(、roles::classify(）临时加进标记表，
                         现存函数里会多出且**仅**多出 2 条 ≥2 阶段接线（接线点实测
                         7 → 9、违规 0 → 2）。但两处性质不同，不能一律当「待清库存」：
                         - driver/helpers.rs 的 collect_package_refs
                           （orchestrator::discover_with_used( + parse →
                           Discovery/PARSE/RoleClassification）：**活代码**，
                           src/driver/arms.rs:626 在调用它 ⇒ 真·跨阶段接线，属待清对象；
                         - frontend/module/orchestrator.rs 的 build_registry
                           （discover( + build_registry_from( → Discovery/Registry）：
                           仓库内**零调用者**，其私有助手 build_registry_from 只被它
                           调用（:381），而 driver/arms.rs 的 Registry 臂自己建表
                           （ModuleRegistry::with_std + extract_module_info +
                           registry.register），不调这两个函数 ⇒ **死代码候选**，
                           正确处置是删除 / 弃用，**不是**登记成遗留库存。它经
                           src/lib.rs 的 pub mod frontend → frontend/mod.rs 的
                           pub mod module → frontend/module/mod.rs 的
                           pub mod orchestrator 暴露为库目标公开项，删除涉及对外 API
                           面，故本判据只陈述性质、不处置它。
                         是否把四类共享原语收进本表属口径扩张，是未决裁决。
                         遗留库存登记在
                         KNOWN_WIRING_SITES（6 条）；重定基清掉的陈旧库存登记
                         在 RETIRED_WIRING_SITES 并以 [进展] 打印（13 条）；
                         P4 建成的声明式调度点登记在 DECLARED_DISPATCH_POINTS
                         并以 [调度点] 打印（不计违规）。
                         新增未登记的接线点 → 违规（验收：故意新增第 6 个
                         入口式接线必须红）。
                         豁免：/tests/ 目录；src 内 #[cfg(test)] mod 模块
                         （范围由**花括号配对**求出）。
                         被测体不含函数签名：签名行会自匹配阶段正则（fn
                         check_project( 命中 TC 的 check_project），虚报阶段。
  A. 同义重复（报告）     同名函数定义出现在 ≥2 个生产文件中（启发式；
                         变体后缀规避如 name_used_as_type_in 由 review 兜底，
                         已登记于 01 路由表 C）。常见助手名按停用表过滤。
  B. 扇出（报告/可选 diff） 默认打印概念族触点文件数基线；
                         --diff <git-ref> 模式下，改动文件集与任一概念族
                         交集 ≥3 → 疑似「一次修改同步改 ≥3 处同义映射」。

C 判据的重定基（2026-10-26）。
  依据（均为仓库内可解析出处）：docs/src/dev/coding-rules.md 禁令三判据 C
  「新增"第 N 个入口 / 表项"而非登记进声明式阶段表 → 违规」、
  docs/src/dev/architecture/02-stage-contract.md §目标设计 1、
  src/driver/stage.rs 的 Stage::Parsing 文档注释、
  docs/src/dev/architecture/09-execution-wbs.md 的 9.12 行。
  旧标定（2026-10-06）是照 P4「统一 Driver」**之前**「六条路径各自手工接线」的
  现状写死的，注释里那句「P4 统一 Driver 后本表随库存一起删除」一直没有兑现，
  于是它把 Driver 自己的阶段表判成了违规。--strict 的 9 处按位置实测：8 处在
  src/driver/，第 9 处是 frontend/module/orchestrator.rs 的 #[cfg(test)] 测试
  函数（由新增的 cfg(test) 豁免消解）。那 8 处按成因分两半：4 处是阶段表 /
  依赖表声明（stage.rs 的 scope、slug，state.rs 的 data_dependencies，mod.rs 的
  dispatch），4 处是把 Parsing 拆成 LEX/PARSE 两阶段造成的单阶段误判
  （parsing_single_file / parsing_check / parsing_lsp / collect_package_refs）。
  P4 之后阶段不再由各入口手工串接，而由 src/driver/stage.rs 的 Stage 阶段表加
  Driver::dispatch 的穷尽 match 声明式驱动。判据因此对齐以下各点：
  1. Parsing = 「词法 + 语法分析（lex → parse）」——src/driver/stage.rs 的
     Stage::Parsing 文档注释与
     docs/src/dev/architecture/02-stage-contract.md §目标设计 1 的阶段表同此
     定义。故 LEX **不是**独立阶段：同一函数内 tokenize + parse 只算 Parsing
     一个阶段。
  2. 死代码族 / 单态化只认**调用形态**：Stage::DeadCodeAnalysis、
     Stage::Monomorphization、"dead-code-analysis" 这类阶段词汇表，以及
     driver/state.rs 的阶段依赖表，都是**声明**而不是接线，不再命中。
  3. src 内 #[cfg(test)] mod 里的函数是测试而不是入口——豁免范围由**花括号
     配对**求出（先 sanitize 再计数），不做「向上 N 行找关键字」的关键字回看：
     回看没有下边界，会整段误豁免或漏豁免。
  4. Driver::dispatch 是 02 §目标设计 1 指定的强制机制（Stage 是编译期可穷举
     枚举，新增变体时 dispatch 的穷尽 match 编译不过——即「登记进声明式阶段
     表」这件事的执行侧），它**就是**判据要求的形态，故单独登记、打印
     [调度点]、不计违规；它**不进** KNOWN_WIRING_SITES（那张表的语义是「待清
     遗留库存」，方向相反）。
  5. 12 个变体的臂调用形态**不手写**：从 dispatch 的穷尽 match 派生（单一源），
     否则「补了 3 条臂形态」仍能被另外两个变体的组合绕过。派生表还要与
     src/driver/*.rs 的 fn 定义对账：臂方法改名或 dispatch 结构变了就报警（见
     stage_arm_markers() 的守护），不静默漏判。阶段形态表每次运行时从源码**重取**
     （不落盘快照，故不会与源码漂移），最近一次重取：2026-10-26。
     覆盖范围要说准，别读成全覆盖：**臂形态 12/12 派生（单一源）；共享原语只覆盖
     6 个变体，另 4 个变体的共享原语形态未收录**（见上条盲区说明，已知未覆盖、
     待裁决）。
  6. 被测体**不含函数签名**：签名行会自匹配阶段正则（fn check_project( 命中 TC
     的 check_project、fn parse( 命中 Parsing 形态），把一个纯声明虚报成一个
     阶段。函数边界仍由 fn_ranges() 的花括号配对求出，只把签名段切掉。

退出码：0 = report-only（默认）；--strict 且违规 > 0 → 1（P9 启用）。
"""
import io
import os
import re
import subprocess
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
SRC = os.path.join(ROOT, 'src')

# 阶段原语标记分两层（标定依据见模块 docstring「C 判据的重定基」）：
# ① 共享阶段原语（本表）—— 可在 Driver 之外被调用的阶段实现调用形态，8 个键：
#    6 个 Stage 变体的实现原语（PARSE / TC / PROOF / DEADCODE / IRGEN / MONO）
#    + Codegen / Exec 两类执行层原语（无对应 Stage 变体）。
# ② 阶段臂形态 —— 12 个 Stage 变体的 self.<臂>(…)，运行时从 Driver::dispatch 的
#    穷尽 match **派生**（单一源，见 stage_arm_markers()）；臂名不手写：只补点对点
#    的几条臂，换个变体组合照样能绕过（2026-10-26 复审的负控即如此）。
# 约定：
# - PARSE 含词法：Stage::Parsing 的定义就是「词法 + 语法分析（lex → parse）」，
#   故 tokenize 与 parse 同属 Parsing，一个函数里两者并存只算 1 个阶段
#   （重定基前的 LEX/PARSE 两标记会把单阶段函数判成跨阶段接线）。
# - DEADCODE / MONO 的共享原语只认调用形态：Stage::DeadCodeAnalysis、
#   Stage::Monomorphization、"dead-code-analysis" 这类**阶段词汇表**以及阶段
#   依赖表不是接线；旧的字面匹配（dead_code / monomorph）会把它们算进去。
SHARED_STAGE_PRIMITIVES = {
    'PARSE': re.compile(
        r'\btokenize\s*\(|Lexer::new|\brun_lexing\b'
        r'|parser::parse\(|\bparse_module\(|\brun_parsing\b'
        r'|\bparse_file\(|Parser::new|\bparse\(&'),
    'TC': re.compile(r'check_module|TypeChecker::new|\brun_typecheck\b|check_source_in_project|\bcheck_project\b|check_single_file'),
    'PROOF': re.compile(r'run_proof_execution|proof_execution'),
    'DEADCODE': re.compile(r'dead_code_analysis\(|DeadCodeAnalyzer'),
    'MONO': re.compile(r'monomorphization\(|monomorphize\(|Monomorphizer::'),
    'IRGEN': re.compile(r'AstToIrGenerator|run_ir_generation|generate_module_ir|generate_ir|ir_gen::'),
    'CODEGEN': re.compile(r'CodegenContext|BytecodeModule::from|BytecodeFile|Translator::new'),
    'EXEC': re.compile(r'Interpreter::new|execute_module|\.execute\('),
}

# 阶段臂形态的键归属：Stage 变体 → 合并后标记表（stage_markers()）里的键。
# 6 个有共享原语的变体并入既有键（同一阶段不出现两个键）；其余 6 个变体的实现
# 只在 Driver 臂内、唯一调用点是 dispatch，除臂形态外没有第二个可调的接线形态，
# 键即变体名。
ARM_KEY_OF_VARIANT = {
    'Parsing': 'PARSE',
    'Typecheck': 'TC',
    'ProofExecution': 'PROOF',
    'DeadCodeAnalysis': 'DEADCODE',
    'IrGeneration': 'IRGEN',
    'Monomorphization': 'MONO',
    'VendorConsistency': 'VendorConsistency',
    'Discovery': 'Discovery',
    'Registry': 'Registry',
    'RoleClassification': 'RoleClassification',
    'GlobalSlotAlloc': 'GlobalSlotAlloc',
    'Linking': 'Linking',
}

# 派生源（单一源）：变体清单来自 Stage::ALL，臂名来自 dispatch 的穷尽 match。
STAGE_SOURCE = ('driver/stage.rs', 'pub const ALL')
DISPATCH_SOURCE = ('driver/mod.rs', 'dispatch')

# 遗留接线点库存（2026-10-26 重定基后剩 6 条；每条都是「仍在跨 ≥2 阶段、但
# 尚未收进 Driver」的入口式接线，属待清欠账而非合规范）。
# 形态：{(文件, 函数名): 阶段集合}；未登记的接线点 = 违规。
# 阶段集合只作登记留档，报告里的 [阶段/…] 一律取**当次实测**值。
KNOWN_WIRING_SITES = {
    # 库入口：编码 + 执行（Driver 之外的 library/CLI 收尾路径）
    ('lib.rs', 'run_with_source_name'): {'CODEGEN', 'EXEC'},
    ('lib.rs', 'run_project'): {'CODEGEN', 'EXEC'},
    # 证明执行机制共享化后的唯一实现（orchestrator 多入口曾同调，02 §漏洞修复）
    ('frontend/proof_execution.rs', 'execute_single_proof_fn'): {'CODEGEN', 'EXEC', 'IRGEN'},
    ('frontend/validate.rs', 'validate_source'): {'PARSE', 'TC'},
    # REPL 与嵌入 std（归 P4 ProgramKind / 06 D 节裁决）
    ('repl/eval.rs', 'evaluate'): {'CODEGEN', 'EXEC'},
    ('std/yx_sources.rs', 'embedded_std_module_info'): {'PARSE', 'TC'},
}

# 2026-10-26 重定基清掉的 13 条陈旧库存（P4「统一 Driver」之后的实测结果）。
# 它们**不再**参与违规判定，但报告必须如实打印 [进展]：库存表悄悄删条目而不留
# 痕，等于把「清零」写成既成事实。若其中某条重新命中 ≥2 阶段（库存回归），判为
# 违规——回归的接线点必须重新登记，不能借清理之名洗成合规。
# 形态：{(文件, 函数名): 清理原因}。
RETIRED_WIRING_SITES = {
    # ① 函数已不存在：P4 把它们并进 Driver，入口侧已无此函数
    ('frontend/pipeline.rs', 'run_typecheck'): '函数已不存在（P4 并入 Driver）',
    ('frontend/pipeline.rs', 'run_ir_generation'): '函数已不存在（P4 并入 Driver）',
    ('frontend/module/orchestrator.rs', 'collect_project_refs'): '函数已不存在（P4 并入 Driver）',
    ('lsp/server.rs', 'update_semantic_db'): 'lsp/server.rs 内该函数已不存在（同名存活者在 src/lsp/world.rs）',
    ('lsp/handlers/diagnostics.rs', 'run_diagnostics'): '函数已不存在（4.2.6 LSP 单文件兜底统一）',
    # ② 函数仍在，但已不再直接调用阶段原语（改为委托 Driver）——实测 0/1 阶段
    # （下一个条目的 check_project / check_source_in_project 早前显示「只剩
    # Typecheck 1 个阶段」是签名自匹配造成的虚报：TC 的 check_project 命中的是
    # 它们自己的 fn 签名行；2026-10-26 修根因后被测体不含签名，实测 0 阶段。）
    ('frontend/pipeline.rs', 'run'): '函数仍在，已不再直接调用阶段原语（委托 Driver）',
    ('frontend/module/orchestrator.rs', 'compile_project'): '函数仍在，已不再直接调用阶段原语（委托 Driver）',
    ('frontend/module/orchestrator.rs', 'compile_embedded_module'): '函数仍在，已不再直接调用阶段原语（委托 Driver）',
    ('frontend/module/orchestrator.rs', 'check_project'): '函数仍在，已不再直接调用阶段原语（委托 Driver）',
    ('frontend/module/orchestrator.rs', 'check_source_in_project'): '函数仍在，已不再直接调用阶段原语（委托 Driver）',
    # ③ 消除 LEX（并入 PARSE）后只剩 Parsing 1 个阶段
    ('lib.rs', 'eval_code'): '只剩 Parsing 1 个阶段（LEX 并入 PARSE）',
    ('frontend/module/orchestrator.rs', 'parse_file'): '只剩 Parsing 1 个阶段（LEX 并入 PARSE）',
    ('lsp/handlers/completion.rs', 'document_symbol_items'): '只剩 Parsing 1 个阶段（LEX 并入 PARSE）',
}

# 声明式调度点（**不是**遗留库存，不计违规）。
# 依据 docs/src/dev/architecture/02-stage-contract.md §目标设计 1：Stage 是编译期
# 可穷举的枚举、禁止运行期注册，「新增变体时 dispatch 的穷尽 match 编译不过」正是
# 这条约束的强制机制——Driver::dispatch 对 Stage 的穷尽 match 就是判据 C 所说
# 「登记进声明式阶段表」的执行侧。把它塞进 KNOWN_WIRING_SITES 会被读成「待清
# 欠账」，方向相反，故另立一张表。
# 形态：{(文件, 函数名): 登记理由}。
DECLARED_DISPATCH_POINTS = {
    ('driver/mod.rs', 'dispatch'): 'P4 建成的唯一阶段调度者；穷尽 match = 声明式阶段表的执行侧',
}

# 判据 A 停用表：通用助手名不参与同名重复报告
A_STOPLIST = {
    'new', 'default', 'from', 'fmt', 'clone', 'drop', 'eq', 'hash', 'build',
    'run', 'get', 'set', 'len', 'name', 'span', 'check', 'parse', 'test',
    'main', 'init', 'push', 'pop', 'insert', 'remove', 'contains', 'iter',
    'next', 'reset', 'clear', 'is_empty', 'to_string', 'as_str', 'ok', 'err',
}

# 判据 B 概念族（标识符词边界；文件集运行时计算）
CONCEPT_FAMILIES = {
    'BinOp 家族': re.compile(r'\bBinOp\b'),
    'UnOp 家族': re.compile(r'\bUnOp\b'),
    'MonoType': re.compile(r'\bMonoType\b'),
    'ConstValue': re.compile(r'\bConstValue\b'),
    'Literal': re.compile(r'\bLiteral\b'),
    'bytecode 运算符（BinaryOp/UnaryOp/CompareOp）': re.compile(r'\b(BinaryOp|UnaryOp|CompareOp)\b'),
}
B_FANOUT_MIN = 3

FN_RE = re.compile(r'\bfn\s+([A-Za-z_]\w*)')
MOD_DECL_RE = re.compile(r'^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+([A-Za-z_]\w*)')
ATTR_LINE_RE = re.compile(r'^\s*#\[')
CHAR_LIT_RE = re.compile(r"^'(\\\\.|[^\\\\'])" + "'")


def sanitize(line):
    """剥掉行注释与字符串/字符/原始字符串字面量（花括号可能藏在这些地方，
    orchestrator.rs:1312 的注释 {x} 曾把函数边界检测整段吞掉）。
    局限：跨行块注释 /* */ 不处理（本仓以 // 文档注释为主）。"""
    out = []
    i = 0
    n = len(line)
    while i < n:
        c = line[i]
        if c == '/' and i + 1 < n and line[i + 1] == '/':
            break
        if c == 'r' and i + 1 < n and line[i + 1] == '"':
            i += 2
            while i < n and line[i] != '"':
                i += 1
            i += 1
            continue
        if c == 'r' and i + 2 < n and line[i + 1] == '#' and line[i + 2] == '"':
            i += 3
            while i + 1 < n and not (line[i] == '"' and line[i + 1] == '#'):
                i += 1
            i += 2
            continue
        if c == '"':
            i += 1
            while i < n and line[i] != '"':
                i += 2 if line[i] == '\\' else 1
            i += 1
            continue
        if c == "'":
            m = CHAR_LIT_RE.match(line[i:])
            if m:
                i += len(m.group(0))
                continue
            # 生命周期 'a：保留（不影响花括号计数）
            out.append(c)
            i += 1
            continue
        out.append(c)
        i += 1
    return ''.join(out)


def read(p):
    with io.open(p, 'r', encoding='utf-8', errors='replace') as f:
        return f.read()


def rs_files():
    for r, dirs, files in os.walk(SRC):
        dirs[:] = [d for d in dirs if d != 'target']
        for fn in files:
            if fn.endswith('.rs'):
                yield os.path.join(r, fn)


def rel(p):
    return os.path.relpath(p, SRC).replace('\\', '/')


def is_test_path(rp):
    return '/tests/' in rp.replace('\\', '/')


def fn_ranges(lines):
    """产出 (函数名, 起始行, 结束行)。签名可跨行；以首个 { 起计深度；
    注释/字符串先经 sanitize 剥离。"""
    out = []
    name = None
    start = 0
    depth = 0
    opened = False
    for i, raw in enumerate(lines):
        line = sanitize(raw)
        if name is None:
            m = FN_RE.search(line)
            if not m:
                continue
            name = m.group(1)
            start = i
            depth = 0
            opened = False
        depth += line.count('{') - line.count('}')
        if '{' in line:
            opened = True
        if opened and depth <= 0:
            out.append((name, start, i))
            name = None
    return out


def fn_body(san, s, e):
    """被测体 = 函数体**去掉签名段**（首个 { 之前的部分）。

    签名若计入被测体，函数名会自匹配阶段正则：fn check_project( 命中 TC 的
    check_project、fn parse( 命中 Parsing 的 parse 形态、fn typecheck_single_file(
    命中 TC 的 check_single_file 子串——把纯声明虚报成阶段。边界仍由
    fn_ranges() 的花括号配对给出（san 须是已 sanitize 的行），这里只切签名。"""
    text = '\n'.join(san[s:e + 1])
    i = text.find('{')
    return text[i + 1:] if i >= 0 else text


def cfg_test_mod_ranges(lines):
    """src 内 #[cfg(test)] mod … { … } 模块的行范围（0 基，含两端）。

    范围由**花括号配对**求出：先逐行 sanitize（剥注释/字符串，避免 {x} 这类注释
    把边界吞掉），再从 mod 声明行起累计 { / } 深度，回到 0 的那一行即模块末行。
    **不做**「向上 N 行找 #[cfg(test)]」的关键字回看——那种判法没有下边界，注释里
    出现过的关键字、或与 mod 相隔很远的属性都会造成整段误豁免。

    属性链：mod 声明行上方紧邻的连续属性行（#[…]）里必须有一条恰为
    #[cfg(test)]。#[cfg(test)] mod foo;（外置文件形态）没有花括号，不产生范围
    ——该文件里的函数由自身模块内的 #[cfg(test)] 或 /tests/ 目录规则覆盖。
    """
    san = [sanitize(x) for x in lines]
    ranges = []
    for i, line in enumerate(san):
        if not MOD_DECL_RE.match(line):
            continue
        is_test_mod = False
        j = i - 1
        while j >= 0 and ATTR_LINE_RE.match(san[j]):
            if san[j].strip().replace(' ', '') == '#[cfg(test)]':
                is_test_mod = True
            j -= 1
        if not is_test_mod:
            continue
        depth = 0
        opened = False
        for k in range(i, len(san)):
            depth += san[k].count('{') - san[k].count('}')
            if '{' in san[k]:
                opened = True
            if opened and depth <= 0:
                ranges.append((i, k))
                break
    return ranges


def stage_variants():
    """src/driver/stage.rs 的 Stage::ALL 变体清单（判据 C 的变体单一源）。"""
    p = os.path.join(SRC, *STAGE_SOURCE[0].split('/'))
    text = read(p)
    i = text.find(STAGE_SOURCE[1])
    if i < 0:
        return []
    seg = text[i:]
    j = seg.find('];')
    if j >= 0:
        seg = seg[:j]
    return re.findall(r'Stage::([A-Za-z_]\w*)', seg)


def dispatch_arms():
    """从 Driver::dispatch 的穷尽 match 派生 {Stage 变体: 臂方法名}。

    按 「Stage::<变体>」 的出现位置切段，段内第一个 「self.<臂>(」 即该变体的臂。
    臂名因此只有一个来源；dispatch 换结构（match 以外）时派生表会空，由
    stage_arm_markers() 的守护报警，而不是静默漏判。"""
    p = os.path.join(SRC, *DISPATCH_SOURCE[0].split('/'))
    lines = read(p).split('\n')
    san = [sanitize(x) for x in lines]
    body = ''
    for name, s, e in fn_ranges(lines):
        if name == DISPATCH_SOURCE[1]:
            body = fn_body(san, s, e)
            break
    arms = {}
    hits = [(m.start(), m.group(1)) for m in re.finditer(r'Stage::([A-Za-z_]\w*)', body)]
    for k, (pos, variant) in enumerate(hits):
        seg = body[pos:hits[k + 1][0]] if k + 1 < len(hits) else body[pos:]
        arm = re.search(r'self\.([A-Za-z_]\w*)\(', seg)
        if arm:
            arms[variant] = arm.group(1)
    return arms


def stage_arm_markers():
    """派生的臂形态标记 + 与源码对账 → (markers, problems)。

    守护（fail-loud；任一不满足都是「声明与事实不符」，按违规处理——否则判据会
    静默漏判）：
      1) Stage::ALL 能解析出变体清单；
      2) 每个变体在 dispatch 的 match 里都有臂；
      3) 每个派生臂名在 src/driver/*.rs 里都有 fn 定义（臂方法改名即报警）。"""
    variants = stage_variants()
    arms = dispatch_arms()
    problems = []
    markers = {}
    if not variants:
        problems.append('%s：解析不到 Stage::ALL 的变体清单（判据 C 失去变体单一源）'
                        % STAGE_SOURCE[0])
    driver_dir = os.path.join(SRC, 'driver')
    defined = set()
    for fn in sorted(os.listdir(driver_dir)):
        if fn.endswith('.rs'):
            defined |= set(re.findall(r'\bfn\s+([A-Za-z_]\w*)', read(os.path.join(driver_dir, fn))))
    for v in variants:
        arm = arms.get(v)
        if not arm:
            problems.append('Stage::%s 在 %s 的 match 里找不到臂（臂被改名 / dispatch 改了结构）'
                            % (v, DISPATCH_SOURCE[0]))
            continue
        if arm not in defined:
            problems.append('Stage::%s 的臂 %s 在 src/driver/*.rs 里没有 fn 定义（臂方法改名）'
                            % (v, arm))
        key = ARM_KEY_OF_VARIANT.get(v, v)
        rx = re.compile(r'self\.' + re.escape(arm) + r'\w*\s*\(')
        markers[key] = re.compile(markers[key].pattern + '|' + rx.pattern) if key in markers else rx
    return markers, problems


def stage_markers():
    """完整标记表 = 共享阶段原语 + 从 dispatch 派生的 12 个变体臂形态。"""
    arm_markers, problems = stage_arm_markers()
    markers = dict(SHARED_STAGE_PRIMITIVES)
    for key, rx in arm_markers.items():
        if key in markers:
            markers[key] = re.compile(markers[key].pattern + '|' + rx.pattern)
        else:
            markers[key] = rx
    return markers, problems


def wiring_sites(markers):
    """函数内 ≥2 个不同阶段标记 → [(file, line, fn, stages)]。
    豁免：/tests/ 目录；src 内 #[cfg(test)] mod 模块内的函数（花括号配对求范围）。
    被测体经 fn_body() 去掉签名段（签名自匹配会虚报阶段）。"""
    sites = []
    for f in rs_files():
        rp = rel(f)
        if is_test_path(rp):
            continue
        lines = read(f).split('\n')
        san = [sanitize(x) for x in lines]
        test_mods = cfg_test_mod_ranges(lines)
        for name, s, e in fn_ranges(lines):
            if any(lo <= s <= hi for lo, hi in test_mods):
                continue
            body = fn_body(san, s, e)
            stages = sorted(k for k, rx in markers.items() if rx.search(body))
            if len(stages) >= 2:
                sites.append((rp, s + 1, name, stages))
    return sites


def dispatch_points(markers):
    """声明的调度点实测 → [(file, line, fn, stages, reason, found)]；不计违规。

    登记失效（文件或函数找不到）是**声明与事实不符**，按违规处理：豁免表本身也
    必须被实测约束，否则它会变成第二张「写死不兑现」的表。"""
    out = []
    for (rp, name), reason in sorted(DECLARED_DISPATCH_POINTS.items()):
        f = os.path.join(SRC, rp)
        hit = None
        if os.path.isfile(f):
            lines = read(f).split('\n')
            san = [sanitize(x) for x in lines]
            for fn, s, e in fn_ranges(lines):
                if fn != name:
                    continue
                body = fn_body(san, s, e)
                hit = (s + 1, sorted(k for k, rx in markers.items() if rx.search(body)))
                break
        out.append((rp, hit[0] if hit else 0, name, hit[1] if hit else [],
                    reason, hit is not None))
    return out


def main():
    strict = '--strict' in sys.argv[1:]
    diff_ref = None
    for i, a in enumerate(sys.argv[1:]):
        if a == '--diff' and i + 2 <= len(sys.argv[1:]):
            diff_ref = sys.argv[i + 2]

    files = sorted(rs_files())
    violations = []

    print('check-fanout: 禁令三（该重构不补丁）门禁报告%s' % (' [STRICT]' if strict else '（report-only）'))
    print('扫描范围: src/（%d 个 .rs）\n' % len(files))

    # ---- C 入口接线 ----
    markers, marker_problems = stage_markers()
    sites = wiring_sites(markers)
    dispatch = dispatch_points(markers)
    print('== C 入口接线点（函数内跨 ≥2 阶段手工接线；判据重定基 2026-10-26：'
          '库存 %d 处 / 已清理 %d 条 / 声明式调度点 %d 处；'
          '阶段标记 %d 键 = 共享原语 %d + 6 个变体专有的臂形态（另 6 个变体的臂形态'
          '并入对应共享原语键））=='
          % (len(KNOWN_WIRING_SITES), len(RETIRED_WIRING_SITES),
             len(DECLARED_DISPATCH_POINTS), len(markers), len(SHARED_STAGE_PRIMITIVES)))
    for p in marker_problems:
        msg = '判据 C 标记守护：%s' % p
        violations.append(msg)
        print('  [违规] ' + msg)
    for rp, line, name, stages, reason, found in dispatch:
        if found:
            print('  [调度点] %s:%d %s  [%s]  %s' % (rp, line, name, '/'.join(stages) or '—', reason))
        else:
            msg = '%s %s  声明式调度点登记失效（文件或函数不存在）' % (rp, name)
            violations.append(msg)
            print('  [违规] ' + msg)
    for rp, line, name, stages in sites:
        key = (rp, name)
        if key in DECLARED_DISPATCH_POINTS:
            continue  # 已在 [调度点] 段打印，不计违规
        if key in KNOWN_WIRING_SITES:
            print('  [库存] %s:%d %s  [%s]' % (rp, line, name, '/'.join(stages)))
        else:
            violations.append('%s:%d %s  新增入口接线 [%s]' % (rp, line, name, '/'.join(stages)))
            print('  [违规] %s:%d %s  [%s]' % (rp, line, name, '/'.join(stages)))
    for (rp, name) in KNOWN_WIRING_SITES:
        if not any((s[0], s[2]) == (rp, name) for s in sites):
            print('  [进展] %s %s 已消失（可从库存移除）' % (rp, name))
    for (rp, name), reason in sorted(RETIRED_WIRING_SITES.items()):
        site = next((s for s in sites if (s[0], s[2]) == (rp, name)), None)
        if site is None:
            print('  [进展] %s %s 已消失（2026-10-26 重定基移出库存：%s）' % (rp, name, reason))
        else:
            msg = '%s:%d %s  库存回归（重定基已移出的库存重新命中 [%s]）' % (
                rp, site[1], name, '/'.join(site[3]))
            violations.append(msg)
            print('  [违规] ' + msg)
    print()

    # ---- A 同名函数多定义 ----
    print('== A 同名函数多定义（生产文件，启发式，人裁决）==')
    fn_defs = defaultdict(set)  # name -> files
    for f in files:
        rp = rel(f)
        if is_test_path(rp):
            continue
        for m in re.finditer(r'^\s*(?:pub(?:\(crate\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_]\w*)', read(f), re.M):
            fn_defs[m.group(1)].add(rp)
    dups = {n: fs for n, fs in fn_defs.items() if len(fs) >= 2 and n not in A_STOPLIST and len(n) >= 4}
    for n, fs in sorted(dups.items()):
        print('  %s: %d 个文件 → %s' % (n, len(fs), '、'.join(sorted(fs))))
    print('  （合计 %d 个同名定义；同名 ≠ 必违规，语义重复由 review 裁决）' % len(dups))
    print()

    # ---- B 扇出 ----
    print('== B 概念族触点基线（扇出事实；--diff 模式下 ≥%d 个同族文件被改 → 疑似扇出）==' % B_FANOUT_MIN)
    family_files = {}
    for fam, rx in CONCEPT_FAMILIES.items():
        s = set()
        for f in files:
            if rx.search(read(f)):
                s.add(rel(f))
        family_files[fam] = s
        print('  %s: %d 个文件' % (fam, len(s)))
    if diff_ref:
        changed = subprocess.run(
            ['git', 'diff', '--name-only', diff_ref, '--', 'src/'],
            cwd=ROOT, capture_output=True, text=True,
        ).stdout.split()
        changed = {c.replace('\\', '/').removeprefix('src/') for c in changed}
        print('\n  --diff %s：改动 %d 个 src 文件' % (diff_ref, len(changed)))
        for fam, fs in family_files.items():
            inter = changed & fs
            if len(inter) >= B_FANOUT_MIN:
                msg = '扇出疑似：%s 命中 %d 个改动文件（%s）' % (fam, len(inter), '、'.join(sorted(inter)[:6]))
                violations.append(msg)
                print('  [违规] ' + msg)
    print()

    print('== 汇总 ==')
    print('接线点实测 %d 处（库存 %d 处；已清理 %d 条；声明式调度点 %d 处不计违规）'
          % (len(sites), len(KNOWN_WIRING_SITES), len(RETIRED_WIRING_SITES), len(dispatch)))
    print('违规：%d 处' % len(violations))
    for v in violations:
        print('  ' + v)
    print('模式：%s' % ('--strict' if strict else 'report-only（P0 引入，退出码恒 0；P9 转硬改用 --strict）'))
    return 1 if (strict and violations) else 0


if __name__ == '__main__':
    if hasattr(sys.stdout, 'reconfigure'):
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    sys.exit(main())
