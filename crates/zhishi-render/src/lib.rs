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
            RenderError::NodeNotFound => write!(
                f,
                "Node.js not found: install Node.js (>= 18) or set ZHISHI_NODE to the node executable"
            ),
            RenderError::ModuleNotFound(m) => write!(
                f,
                "Node module `{m}` not found: install it with `npm install` in the zhishi package, \
                 or set ZHISHI_NODE_MODULES to a node_modules directory containing it"
            ),
            RenderError::Failed { stderr, code } => {
                let code = code.map_or_else(|| "signal".to_string(), |c| c.to_string());
                write!(f, "renderer failed (exit {code}): {stderr}")
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

    // `-o out/01.png` 是文档里的常见写法，父目录不存在时先建出来。
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RenderError::Io(format!("create {}: {e}", parent.display())))?;
        }
    }

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
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if let Some(m) = missing_module(&stderr) {
            return Err(RenderError::ModuleNotFound(m));
        }
        return Err(RenderError::Failed {
            stderr,
            code: output.status.code(),
        });
    }
    Ok(())
}

fn find_node() -> Result<PathBuf, RenderError> {
    // npm 包装器注入的路径优先，其次 PATH。
    if let Some(p) = env_executable("ZHISHI_NODE")? {
        return Ok(p);
    }
    find_in_path("node").ok_or(RenderError::NodeNotFound)
}

/// 读取一个"指向可执行文件"的环境变量：可以是绝对路径，也可以是名字（走 PATH）。
/// 变量存在但解析不出来时报错，避免静默回退掩盖配置笔误。
fn env_executable(key: &str) -> Result<Option<PathBuf>, RenderError> {
    let Some(raw) = std::env::var_os(key) else {
        return Ok(None);
    };
    if raw.is_empty() {
        return Ok(None);
    }
    if let Some(found) = find_in_path(&raw.to_string_lossy()) {
        return Ok(Some(found));
    }
    Err(RenderError::Io(format!(
        "{key}={} is not an executable file",
        Path::new(&raw).display()
    )))
}

fn find_node_modules() -> Option<PathBuf> {
    let raw = std::env::var_os("ZHISHI_NODE_MODULES")?;
    let dir = PathBuf::from(raw);
    dir.join("mathjax-full").exists().then_some(dir)
}

/// 在 PATH 中查找可执行文件。
///
/// 自己遍历 PATH，不依赖外部的 `which`/`where` 命令，因此各平台行为一致。
fn find_in_path(name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let as_path = Path::new(name);
    // 显式路径（`./node`、`/usr/bin/node`）直接判定，不查 PATH。
    if as_path.is_absolute() || as_path.components().count() > 1 {
        return as_path.is_file().then(|| as_path.to_path_buf());
    }
    find_in_paths(name, &std::env::var_os("PATH")?)
}

fn find_in_paths(name: &str, path_var: &std::ffi::OsStr) -> Option<PathBuf> {
    let candidates = executable_candidates(name);
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        for cand in &candidates {
            let full = dir.join(cand);
            if is_executable(&full) {
                return Some(full);
            }
        }
    }
    None
}

/// 在 PATH 各目录中依次尝试的文件名。
///
/// Windows 下按 `PATHEXT` 补扩展名；`.bat`/`.cmd`/`.ps1` 这类脚本被
/// `CreateProcess` 直接拒绝，故只保留 `.exe`/`.com`。
#[cfg(windows)]
fn executable_candidates(name: &str) -> Vec<String> {
    let mut out = vec![name.to_string()];
    if Path::new(name).extension().is_none() {
        let pathext = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE".to_string());
        for ext in pathext.split(';') {
            let ext = ext.trim().to_ascii_lowercase();
            if ext == ".exe" || ext == ".com" {
                out.push(format!("{name}{ext}"));
            }
        }
        if !out.iter().any(|c| c.to_ascii_lowercase().ends_with(".exe")) {
            out.push(format!("{name}.exe"));
        }
    }
    out
}

#[cfg(not(windows))]
fn executable_candidates(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// 从 Node 的报错中识别缺失的模块名，用于给出可操作的提示。
fn missing_module(stderr: &str) -> Option<String> {
    const MARK: &str = "Cannot find module '";
    let rest = &stderr[stderr.find(MARK)? + MARK.len()..];
    let spec = &rest[..rest.find('\'')?];
    // 报错可能是 `mathjax-full/js/mathjax.js`，真正缺的是顶层包。
    Some(top_level_package(spec))
}

fn top_level_package(spec: &str) -> String {
    let mut parts = spec.split('/');
    let first = parts.next().unwrap_or(spec);
    if first.starts_with('@') {
        match parts.next() {
            Some(second) => format!("{first}/{second}"),
            None => first.to_string(),
        }
    } else {
        first.to_string()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("zhishi-render-test-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn set_executable(path: &Path, executable: bool) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = if executable { 0o755 } else { 0o644 };
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        #[cfg(not(unix))]
        {
            let _ = (path, executable);
        }
    }

    #[test]
    fn finds_executable_in_path() {
        let dir = temp_dir("hit");
        let exe = dir.join("zhishi-fake-node");
        std::fs::write(&exe, b"").unwrap();
        set_executable(&exe, true);

        let found = find_in_paths("zhishi-fake-node", OsStr::new(&dir));
        assert_eq!(found, Some(exe));
    }

    #[test]
    fn skips_non_executable_file_on_unix() {
        let dir = temp_dir("noexec");
        let file = dir.join("zhishi-fake-node");
        std::fs::write(&file, b"").unwrap();
        set_executable(&file, false);

        let found = find_in_paths("zhishi-fake-node", OsStr::new(&dir));
        #[cfg(unix)]
        assert_eq!(found, None);
        #[cfg(not(unix))]
        assert_eq!(found, Some(file));
    }

    #[test]
    fn does_not_find_absent_name() {
        let dir = temp_dir("miss");
        assert_eq!(find_in_paths("zhishi-not-here", OsStr::new(&dir)), None);
    }

    #[test]
    fn empty_path_entries_are_ignored() {
        let dir = temp_dir("empty");
        let exe = dir.join("zhishi-fake-node");
        std::fs::write(&exe, b"").unwrap();
        set_executable(&exe, true);

        // 空路径项（POSIX 上意为当前目录）不应被当作命中。
        let joined = std::env::join_paths([OsStr::new(""), dir.as_os_str()]).unwrap();
        assert_eq!(find_in_paths("zhishi-fake-node", &joined), Some(exe));
    }

    #[test]
    fn explicit_path_is_used_directly() {
        let dir = temp_dir("explicit");
        let exe = dir.join("node");
        std::fs::write(&exe, b"").unwrap();
        set_executable(&exe, true);

        assert_eq!(find_in_path(&exe.to_string_lossy()), Some(exe.clone()));
        assert_eq!(find_in_path(&dir.join("missing").to_string_lossy()), None);
    }

    #[test]
    fn parses_missing_node_module() {
        let stderr = "\
node:internal/modules/cjs/loader:1143
Error: Cannot find module 'mathjax-full/js/mathjax.js'
Require stack:
- /tmp/zhishi-render/render_math.cjs";
        assert_eq!(missing_module(stderr).as_deref(), Some("mathjax-full"));

        let scoped = "Error: Cannot find module '@resvg/resvg-js/index.js'";
        assert_eq!(missing_module(scoped).as_deref(), Some("@resvg/resvg-js"));

        assert_eq!(missing_module("some unrelated failure"), None);
    }
}
