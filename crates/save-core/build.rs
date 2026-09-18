use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=templates");
    let mut paths: Vec<_> = fs::read_dir("templates")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let entries = paths
        .iter()
        .map(|p| format!("{:?}", fs::read_to_string(p).unwrap()))
        .collect::<Vec<_>>()
        .join(",");
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("templates.rs"),
        format!("const TEMPLATES: &[&str] = &[{entries}];"),
    )
    .unwrap();
}
