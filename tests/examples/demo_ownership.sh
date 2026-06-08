#!/bin/bash
# PLDI SRC Demo Script — YaoXiang Token System
# 展示核心特性：Move、&T、&mut T、spawn、元组解构

echo "=== YaoXiang Token System Demo ==="
echo ""

echo "--- 1. Move 语义 ---"
cargo run --quiet -- run tests/yaoxiang/08-ownership/move_basic.yx
echo ""

echo "--- 2. &T 不可变借用 (Dup) ---"
cargo run --quiet -- run tests/yaoxiang/08-ownership/borrow_immutable.yx
echo ""

echo "--- 3. &mut T 可变借用 (Linear) ---"
cargo run --quiet -- run tests/yaoxiang/08-ownership/borrow_mutable.yx
echo ""

echo "--- 4. 令牌作为结构体字段 ---"
cargo run --quiet -- run tests/yaoxiang/08-ownership/token_in_struct.yx
echo ""

echo "--- 5. Spawn 块并行执行 ---"
cargo run --quiet -- run tests/yaoxiang/09-concurrency/spawn_basic.yx
echo ""

echo "--- 6. Spawn + ref 共享所有权 ---"
cargo run --quiet -- run tests/yaoxiang/09-concurrency/spawn_ref.yx
echo ""

echo "--- 7. 元组解构 ---"
cargo run --quiet -- run tests/yaoxiang/09-concurrency/test_destructure.yx
echo ""

echo "--- 8. 变量遮蔽检测 (E2002) ---"
cargo run --quiet -- run tests/yaoxiang/10-errors/shadow_err.yx 2>&1 || true
echo ""

echo "=== All demos complete ==="
