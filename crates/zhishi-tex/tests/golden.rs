//! 黄金样例 harness：examples/01-*.zsh 应产出与同名 .tex 一致的 LaTeX。
//!
//! M0 阶段 parse/to_tex 未实现，此测试故意全红 —— 作为 M1–M3 的冲刺目标。

use std::fs;
use std::path::{Path, PathBuf};

fn examples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .expect("examples/ directory must exist")
}

#[test]
fn golden_examples() {
    let dir = examples_dir();
    let mut cases: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read examples/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "zsh").unwrap_or(false))
        .collect();
    cases.sort();

    assert!(
        cases.len() >= 20,
        "expected at least 20 golden .zsh examples, found {}",
        cases.len()
    );

    let mut failures = Vec::new();

    for zsh_path in &cases {
        let stem = zsh_path.file_stem().unwrap().to_string_lossy().to_string();
        let tex_path = zsh_path.with_extension("tex");
        let src = fs::read_to_string(zsh_path).expect("read zsh");
        let expected_raw = fs::read_to_string(&tex_path).unwrap_or_default();
        let expected = zhishi_tex::normalize_tex(&expected_raw);

        match zhishi_core::parse(&src) {
            Err(diag) => {
                failures.push(format!("{stem}: parse failed — {}", diag.message));
            }
            Ok(formula) => {
                let got = zhishi_tex::normalize_tex(&zhishi_tex::to_tex(&formula));
                if got != expected {
                    failures.push(format!(
                        "{stem}: tex mismatch\n  expected: {expected}\n  got:      {got}"
                    ));
                }
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "golden examples failing ({}/{}):\n{}",
            failures.len(),
            cases.len(),
            failures.join("\n")
        );
    }
}
