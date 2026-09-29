# 直式（ZhiShi）

数学公式描述语言：写法贴近课堂读法，编译为 TeX，再渲染成 SVG/PNG。

```text
(-b ± sqrt(b^2 - 4*a*c)) / (2*a)
→  \frac{-b \pm \sqrt{b^{2} - 4 a c}}{2 a}
→  图片
```

- 语言规范：`docs/直式语言规范草案.md`
- 实现计划：`docs/直式实现计划.md`（Rust）

## 构建与测试

```bash
cargo build
cargo test                 # 22 单测 + 20 黄金样例
cargo run -p zhishi-cli -- check examples/01-quadratic.zsh
cargo run -p zhishi-cli -- tex   examples/01-quadratic.zsh
cargo run -p zhishi-cli -- render examples/01-quadratic.zsh -o out/01.png
```

`check`/`tex` 零渲染依赖。`render` 需要 Node.js（>= 18）以及 MathJax/sharp：
直接跑 cargo 二进制时要自己指定依赖目录 `ZHISHI_NODE_MODULES`；
走 npm 包装器（`node npm/bin/zsc.js`）则会自动注入，但前提是先按下一节打包好 vendor 二进制。

```bash
npm install --prefix npm
ZHISHI_NODE_MODULES=$PWD/npm/node_modules \
  cargo run -p zhishi-cli -- render examples/01-quadratic.zsh -o out/01.png
```

## npm 安装（推荐给使用者）

CLI 通过 npm 分发，Node 依赖由 npm 管理：

```bash
# 从源码打包安装（开发期）
cargo build --release -p zhishi-cli
node npm/scripts/copy-binary.js
npm install -g ./npm

# 发布后
npm install -g zhishi
zsc render formula.zsh -o formula.png
```

npm 包装器（`npm/bin/zsc.js`）会自动定位 `vendor/<platform>-<arch>/zsc`
并注入 `ZHISHI_NODE` / `ZHISHI_NODE_MODULES`，无需手动配环境。

## 仓库结构

```text
crates/zhishi-core     词法、语法、AST、诊断、检查
crates/zhishi-tex      AST → LaTeX
crates/zhishi-render   TeX → SVG/PNG（MathJax/Node）
crates/zhishi-cli      zsc
examples/              20 个试金石（.zsh + 期望 .tex）
```

## 语言速查

| 想写 | 直式 | 说明 |
|---|---|---|
| 分数 | `a / b` | 嵌套自行加括号：`1/(1+1/x)` |
| 根号 | `sqrt(x)`，`root(n, x)` | |
| 上下标 | `x^2` `x_i` `x_i^2` `x_(i+1)` | `_` 不进标识符 |
| 大算符 | `sum_(i=1)^n` `prod_(k=1)^n` `int_a^b` `lim_(x->0)` | |
| 导数 | `y'` `f'(x)` `f''(x)`，`dy/dx`，`partial^2 u/partial x^2` | |
| 括号 | `(a+b)(a-b)` | 自动伸缩，不用管大小 |
| 绝对值/范数 | `` `|x|` ``，`` `||x||` `` | |
| 矩阵 | `matrix(1, 2; 3, 4)`，`vmatrix(a, b; c, d)` | `;` 分行，`,` 分列 |
| 分段 | `cases{ x^2, x > 0; 0, otherwise }` | |
| 对齐方程 | `align{ x+y &= 5; x-y &= 1 }` | |
| 集合 | `{x mid x > 0}` | 分隔统一用 `mid` |
| 二项式 | `binom(n, k)` | |
| 希腊字母 | `alpha` `beta` `pi` `theta` … | 保留字，整词匹配 |
| 常量 | `oo`（∞）`pi` `e` `'i`（虚数单位） | `i` 是普通变量 |
| 修饰 | `bar(x)` `hat(x)` `vec(v)` `bold(A)` `overline(AB)` | |
| 文本 | `"otherwise"` | 直立体 |
| 函数声明 | `fn f(x, y) = x^2 + y^2` | 声明后 `f(a,b)` 是调用而非乘法 |
| 乘法 | `2x` `3(x+1)` `a * b` | 并列即乘；`*` 与并列都输出并列 |
| 逻辑 | `and` `or` `not` | 只用单词，不用符号 |

## 三条硬规则（规范 §5）

1. **`a(b)` 默认是乘法**。`sin(x)` 这类内置名、或 `fn` 登记过的名字才是调用。
2. **`/` 与乘同级左结合**。`x / y z` = `(x/y)*z`；分母是乘积请写 `x / (y*z)`。
3. **`^` 右结合、`_` 取单原子**。`x^i+1` = `(x^i)+1`；指数是和请写 `x^(i+1)`。

## CLI

```text
zsc check <file.zsh>          语法检查（--json）
zsc tex   <file.zsh>          输出 LaTeX
zsc ast   <file.zsh>          打印 AST
zsc render <file.zsh> -o out.png
        --font-size 24 --margin 10 --opaque
```

## 试金石

`examples/*.zsh` 是规范 §6 的 20 个例子，同名 `.tex` 是黄金期望；
`cargo test -p zhishi-tex` 做文本比对，渲染结果在 `out/`（已忽略）。
