//! 直式渲染后端：LaTeX 串 → SVG / PNG。
//!
//! v0.1 通过 Node + MathJax 出 SVG，再用 sharp 光栅化 PNG。
//! 渲染与语言层严格分离：`check`/`tex` 不依赖本 crate。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const RENDER_SCRIPT: &str = include_str!("../assets/render_math.cjs");

#[derive(Debug, Clone)]
pub struct RenderOpts {
    pub format: RenderFormat,
    pub font_size: f32,
    pub margin: f32,
    pub transparent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    Svg,
    Png,
}

impl Default for RenderOpts {
    fn default() -> Self {
        Self {
            format: RenderFormat::Png,
            font_size: 24.0,
            margin: 10.0,
            transparent: true,
        }
    }
}

#[derive(Debug)]
pub enum RenderError {
    NodeNotFound,
    ModuleNotFound(String),
    Failed { stderr: String, code: Option<i32> },
    Io(String),
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::NodeNotFound => {
                write!(f, "Node.js not found (set MIMO_NODE or put node on PATH)")
            }
            RenderError::ModuleNotFound(m) => {
                write!(f, "Node module not found: {m} (set MIMO_NODE_MODULES)")
            }
            RenderError::Failed { stderr, code } => {
                write!(f, "renderer failed (exit {code:?}): {stderr}")
            }
            RenderError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// 将 LaTeX 数学串渲染到 `out` 文件（按扩展名或 opts.format）。
pub fn render_to_file(tex: &str, out: &Path, opts: &RenderOpts) -> Result<(), RenderError> {
    let format = match out
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .as_deref()
    {
        Some("svg") => RenderFormat::Svg,
        Some("png") | None => RenderFormat::Png,
        Some(other) => {
            return Err(RenderError::Io(format!(
                "unsupported output extension {other}"
            )))
        }
    };

    let node = find_node()?;
    let script = write_script()?;

    let mut cmd = Command::new(&node);
    cmd.arg(&script)
        .arg("--out")
        .arg(out)
        .arg("--format")
        .arg(match format {
            RenderFormat::Svg => "svg",
            RenderFormat::Png => "png",
        })
        .arg("--font-size")
        .arg(opts.font_size.to_string())
        .arg("--margin")
        .arg(opts.margin.to_string());
    if opts.transparent {
        cmd.arg("--transparent");
    }

    if let Some(mods) = find_node_modules() {
        cmd.env("NODE_PATH", &mods);
    }

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| RenderError::Io(format!("spawn node: {e}")))?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| RenderError::Io("stdin".into()))?;
        stdin
            .write_all(tex.as_bytes())
            .map_err(|e| RenderError::Io(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| RenderError::Io(e.to_string()))?;

    if !output.status.success() {
        return Err(RenderError::Failed {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            code: output.status.code(),
        });
    }
    Ok(())
}

fn find_node() -> Result<PathBuf, RenderError> {
    // npm 包装器注入的路径优先，其次 MiMo 运行时，再次 PATH
    for key in ["ZHISHI_NODE", "MIMO_NODE"] {
        if let Ok(p) = std::env::var(key) {
            if Path::new(&p).exists() {
                return Ok(PathBuf::from(p));
            }
        }
    }
    for name in ["node", "node.exe"] {
        if let Ok(p) = which(name) {
            return Ok(p);
        }
    }
    Err(RenderError::NodeNotFound)
}

fn find_node_modules() -> Option<PathBuf> {
    for key in ["ZHISHI_NODE_MODULES", "MIMO_NODE_MODULES"] {
        if let Ok(p) = std::env::var(key) {
            if Path::new(&p).join("mathjax-full").exists() {
                return Some(PathBuf::from(p));
            }
        }
    }
    None
}

fn which(name: &str) -> Result<PathBuf, RenderError> {
    let out = Command::new("where")
        .arg(name)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| RenderError::Io(e.to_string()))?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .find(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .ok_or(RenderError::NodeNotFound)
}

fn write_script() -> Result<PathBuf, RenderError> {
    let dir = std::env::temp_dir().join("zhishi-render");
    std::fs::create_dir_all(&dir).map_err(|e| RenderError::Io(e.to_string()))?;
    let path = dir.join("render_math.cjs");
    // 内容不变则跳过写入
    if std::fs::read_to_string(&path)
        .map(|s| s == RENDER_SCRIPT)
        .unwrap_or(false)
    {
        return Ok(path);
    }
    std::fs::write(&path, RENDER_SCRIPT).map_err(|e| RenderError::Io(e.to_string()))?;
    Ok(path)
}
