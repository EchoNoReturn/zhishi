# zhishi

直式（ZhiShi）数学公式语言 CLI。把贴近课堂读法的公式写法编译成 LaTeX / SVG / PNG。

```bash
npm install -g zhishi   # 或从仓库 npm/ 目录本地安装
zsc render formula.zsh -o formula.png
zsc tex formula.zsh
zsc check formula.zsh
```

依赖：Node.js ≥ 18（渲染用）。`check` / `tex` 不需要额外环境。

仓库与文档：https://github.com/EchoNoReturn/zhishi
