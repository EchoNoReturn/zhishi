# 直式（ZhiShi）

数学公式描述语言：写法贴近课堂读法，编译为 TeX 再渲染成图片。

- 语言规范：`docs/直式语言规范草案.md`
- 实现计划：`docs/直式实现计划.md`（Rust）

## 快速开始

```bash
cargo build
cargo test                 # 黄金样例（M0 阶段预期为红，M3 冲绿）
cargo run -p zhishi-cli -- check examples/01-quadratic.zsh
cargo run -p zhishi-cli -- tex  examples/01-quadratic.zsh
```

## 仓库结构

```
crates/zhishi-core   # 词法、语法、AST、诊断、检查
crates/zhishi-tex    # AST → LaTeX
crates/zhishi-cli    # zsc
examples/            # 20 个试金石（.zsh + 期望 .tex）
```

## 语言一瞥

```
(-b ± sqrt(b^2 - 4*a*c)) / (2*a)
sum_(i=1)^n i^2 = n*(n+1)*(2*n+1)/6
f(x) = cases{ x^2, x > 0; 0, x <= 0 }
exp('i * pi) + 1 = 0
```
