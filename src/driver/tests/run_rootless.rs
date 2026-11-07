//! rootless（standalone / Script 语义）多文件 **run** 形态的编排钉测试 — 基于
//! WBS 4.10.3（4.2.5 裁决 A 的 run 半边）与 #413 T4 入口语义。
//!
//! 形态：无项目根（无 yaoxiang.toml）的入口文件经 Program{MultiFile} 编排运行。
//! 与 check 半边（check_rootless_mono.rs）同一家族的 run 侧判据：
//! - Script 无入口概念——Linking 不得给 rootless 合并 IR 设 entry_function
//!   （#413 T4：顶层语句即程序主体，自动调 main 会与显式 main() 双跑，#356）；
//! - Script 语义保持——顶层可执行语句不报 E3023（ir_gen 的 script_mode 由
//!   rootless 驱动，与限定名键解耦，同 3.4.8 ③ 的 check 半边）；
//! - Bin 形态对照——带 manifest 的项目入口语义不变（entry_function 照旧设置），
//!   证明 Script 侧断言不是空转。
//!
//! #467（WBS 4.10.4）：多单元初始化序列按 `use` 依赖拓扑序拼接——被依赖单元
//! 的初始化先于导入它的单元；无依赖边的单元保持发现顺序（发现期按路径字典序
//! 排序）；模块导入环内按路径字典序（确定性约定，不是语义保证）。拼接序在
//! Linking 臂（两形态共用）里生效，故 Script 与 Bin 同源——Bin 侧只额外保留
//! `entry_function`（入口调用仍由运行期排在全部 init 之后）。
//!
//! 判据来源：WBS 4.10.3 / 4.10.4；#413（T4 决议）；#356（防双跑）；#467
//! （初始化顺序）；RFC-029f 角色模型。
