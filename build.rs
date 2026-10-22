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

#[path = "langs/lib_python/modules/manifest.rs"]
mod library_sources;

fn prepared_libraries(versions: &serde_json::Map<String, Value>) {
    let base: Value = serde_json::from_str(&fs::read_to_string("langs/python.json").unwrap()).unwrap();
    let mut definitions = vec![base.clone()];
    for item in versions.values() {
        let labels: Value = serde_json::from_str(&fs::read_to_string(item["label_overlay"].as_str().unwrap()).unwrap()).unwrap();
        let mut definition = base.clone();
        for (label, setting) in labels.as_object().unwrap() {
            definition.as_object_mut().unwrap().insert(label.clone(), setting.clone());
        }
        if !definitions.contains(&definition) { definitions.push(definition); }
    }
    let mut inventory = format!("{} sources, {} aliases\n", library_sources::MODULES.len(), library_sources::MODULE_ALIASES.len());
    let mut output = String::from("pub fn install(kernel: &str) {\n");
    for definition in definitions {
        let definition = definition.to_string();
        for kernel in ["stack8", "microcode7"] {
            let mut records = String::new();
            let mut source_rows = String::new();
            let mut count = 0;
            for (name, source, _) in library_sources::MODULES {
                // Reference modules are inputs, not reusable startup libraries.
                if name.starts_with("test.") && !name.starts_with("test.support") { continue; }
                // shortcut: bound prepared library data to fit Lambda; include large IO/archive startup modules.
                if source.len() > 16_384 && !name.starts_with("pathlib.") && !["__main__", "unittest", "_pyio", "tarfile", "pathlib", "_abc", "_codec_runtime", "_codecs", "_collections", "_collections_abc", "_colorize", "_io", "_json", "_operator", "_py_abc", "_pydatetime", "_runtime_import", "_sre", "_sre_validation", "_stat", "_thread", "_types", "_typing_runtime", "_weakref", "_weakrefset", "abc", "annotationlib", "argparse", "ast", "builtins", "codecs", "collections", "collections.abc", "copy", "copyreg", "dataclasses", "enum", "errno", "functools", "genericpath", "gettext", "inspect", "itertools", "json", "json.decoder", "json.encoder", "json.scanner", "json.tool", "keyword", "operator", "os", "os.path", "posix", "posixpath", "re", "re._casefix", "re._compiler", "re._constants", "re._parser", "reprlib", "runtime_adapters.native_test_support", "stat", "sys", "test.support", "test.support.script_helper", "threading", "types", "weakref", "tempfile", "shutil", "random", "logging", "contextlib", "textwrap", "sysconfig", "py_compile", "zipfile", "gzip", "bz2", "lzma", "ntpath", "test.support.os_helper", "test.support.import_helper", "locale", "importlib._bootstrap", "importlib._bootstrap_external", "_py_warnings"].contains(&name) { continue; }
                let prepared = std::panic::catch_unwind(|| match kernel {
                    "stack8" => lumen_stack8::compile::prepare_library(&definition, source),
                    _ => lumen_microcode7::build::prepare_library(&definition, source),
                }).ok().flatten();
                if let Some((key, proof, encoded)) = prepared {
                    let packed = lumen_lzma::encode(&proof, 6, 4).expect("bundled token proof compression");
                    let octets = packed.iter().map(|byte| format!("\\x{byte:02x}")).collect::<String>();
                    let metadata = lumen_lzma::encode(encoded.as_bytes(), 6, 4).expect("bundled scope compression");
                    let metadata_bytes = metadata.iter().map(|byte| format!("\\x{byte:02x}")).collect::<String>();
                    records.push_str(&format!("({key}, b\"{octets}\", b\"{metadata_bytes}\"),\n"));
                    let source_key = match kernel {
                        "stack8" => lumen_stack8::compile::source_stamp(source),
                        _ => lumen_microcode7::build::source_key(source),
                    };
                    let compressed_source = lumen_lzma::encode(source.as_bytes(), 6, 4).expect("bundled source compression");
                    let source_bytes = compressed_source.iter().map(|byte| format!("\\x{byte:02x}")).collect::<String>();
                    source_rows.push_str(&format!("({source_key}, b\"{source_bytes}\", {count}),\n"));
                    count += 1;
                }
            }
            inventory.push_str(&format!("prepared {count} bundled library surveys for {kernel}\n"));
            let module = if kernel == "stack8" { "lumen_stack8::compile" } else { "lumen_microcode7::build" };
            output.push_str(&format!("if kernel == {kernel:?} {{ {module}::register_prepared({definition:?}, &[{records}], &[{source_rows}]); }}\n"));
        }
    }
    output.push_str("}\n");
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("prepared_libraries.rs"), output).unwrap();
    fs::write(PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("prepared_inventory.txt"), inventory).unwrap();
    watch(Path::new("langs/python.json"));
    fn watch_sources(directory: &Path) {
        watch(directory);
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { watch_sources(&path); }
            else if path.extension().is_some_and(|suffix| suffix == "py" || suffix == "rs") { watch(&path); }
        }
    }
    watch_sources(Path::new("langs/lib_python/modules"));
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
    prepared_libraries(versions);
}
