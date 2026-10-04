use std::{env, fs, path::{Path, PathBuf}};
use serde_json::Value;

fn watch(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}

fn modules(root: &Path, here: &Path, layout: &Value, out: &mut String) {
    watch(here);
    let mut entries: Vec<_> = fs::read_dir(here).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for file in entries {
        if file.is_dir() {
            modules(root, &file, layout, out);
        } else if file.extension().is_some_and(|e| e == "py") {
            let relative = file.strip_prefix(root).unwrap().to_str().unwrap();
            if layout["adapters"].as_object().is_some_and(|a| a.values().any(|v| v.as_str() == Some(relative))) { continue; }
            let mut name = relative.strip_suffix(".py").unwrap().replace('/', ".");
            if let Some(package) = name.strip_suffix(".__init__") { name = package.to_string(); }
            let absolute = fs::canonicalize(&file).unwrap();
            let location = layout["files"][&name].as_str().unwrap_or(relative);
            let expression = if let Some(adapter) = layout["adapters"][&name].as_str() {
                let adapter = fs::canonicalize(root.join(adapter)).unwrap();
                watch(&adapter);
                format!("concat!(include_str!({:?}), \"\n\", include_str!({:?}))", absolute.to_str().unwrap(), adapter.to_str().unwrap())
            } else { format!("include_str!({:?})", absolute.to_str().unwrap()) };
            out.push_str(&format!("({name:?}, {expression}, {location:?}),\n"));
        }
    }
}

fn main() {
    let table = Path::new("langs/python/versions.json");
    watch(table);
    let data: Value = serde_json::from_str(&fs::read_to_string(table).unwrap()).unwrap();
    assert_eq!(data["window"], 2, "Python supports a two-series window");
    let versions = data["versions"].as_object().expect("version table");
    assert!(!versions.is_empty() && versions.len() <= 2, "register one pin per series, at most two series");
    let mut generated = String::from("pub static VERSIONS: &[PythonVersion] = &[\n");
    for (series, item) in versions {
        let field = |name| item[name].as_str().unwrap_or_else(|| panic!("{series}: missing {name}"));
        let release = field("release");
        let numbers: Vec<u32> = release.split('.').map(|n| n.parse().expect("release number")).collect();
        assert_eq!(numbers.len(), 3);
        assert_eq!(series, &format!("{}.{}", numbers[0], numbers[1]));
        assert_eq!(field("tag"), format!("v{release}"));
        for key in ["commit", "release_date", "tests"] { assert!(!field(key).is_empty()); }
        let label = Path::new(field("label_overlay"));
        watch(label);
        let labels: Value = serde_json::from_str(&fs::read_to_string(label).unwrap()).unwrap();
        assert!(labels.as_object().expect("label overlay object").keys().all(|k| k.starts_with("ext.")), "overlays replace extension labels only");
        let absolute = fs::canonicalize(label).unwrap();
        let overlay = Path::new(field("library_overlay"));
        let mut sources = String::new();
        let layout_path = overlay.join("manifest-layout.json");
        watch(&layout_path);
        let layout: Value = if layout_path.is_file() { serde_json::from_str(&fs::read_to_string(&layout_path).unwrap()).unwrap() } else { serde_json::json!({}) };
        modules(overlay, overlay, &layout, &mut sources);
        let mut aliases = String::new();
        if let Some(entries) = layout["aliases"].as_object() {
            for (name, member) in entries { aliases.push_str(&format!("({name:?}, {:?}),", member.as_str().unwrap())); }
        }
        generated.push_str(&format!("PythonVersion {{ release: {release:?}, tests: {:?}, numbers: {numbers:?}, labels: include_str!({:?}), modules: &[{sources}], aliases: &[{aliases}] }},\n", field("tests"), absolute.to_str().unwrap()));
    }
    generated.push_str("];\n");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("python_versions.rs"), generated).unwrap();
}
