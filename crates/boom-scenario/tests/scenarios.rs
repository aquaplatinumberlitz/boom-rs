//! Chạy mọi scenario trong `fixtures/scenarios/**.toml`.
//!
//! `active` phải qua; `pending` (feature chưa làm) được chạy và in kết quả nhưng không làm fail.
//! Lọc: `SCENARIO=movement cargo test -p boom-scenario -- --nocapture`.

use std::path::{Path, PathBuf};

use boom_scenario::scenario::{Status, run_file};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.expect("đọc thư mục").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "toml") {
            out.push(p);
        }
    }
}

#[test]
fn scenarios() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/scenarios");
    let mut files = Vec::new();
    collect(&root, &mut files);
    let filter = std::env::var("SCENARIO").unwrap_or_default();
    let (mut passed, mut pending_open, mut errors) = (0, 0, Vec::new());
    for file in files
        .iter()
        .filter(|f| f.to_string_lossy().contains(&filter))
    {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .display()
            .to_string();
        match run_file(file) {
            Err(e) => errors.push(format!("{rel}: không chạy được: {e}")),
            Ok(r) if r.failures.is_empty() => {
                passed += 1;
                if r.status == Status::Pending {
                    println!("GỢI Ý {rel}: pending nhưng đã qua — chuyển status = \"active\"");
                }
            }
            Ok(r) if r.status == Status::Pending => {
                pending_open += 1;
                println!("PENDING {rel} ({}) {:?}", r.name, r.features);
                for f in &r.failures {
                    println!("    {f}");
                }
            }
            Ok(r) => errors.push(format!(
                "{rel} ({}) {:?}:\n    {}",
                r.name,
                r.features,
                r.failures.join("\n    ")
            )),
        }
    }
    println!(
        "scenario: {passed} qua, {pending_open} pending chưa qua, {} lỗi",
        errors.len()
    );
    assert!(errors.is_empty(), "scenario lỗi:\n{}", errors.join("\n"));
}
