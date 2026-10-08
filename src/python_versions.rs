//! Python selection belongs to the host; kernels consume the merged labels.
use std::{env, fs};
use serde_json::Value;

pub struct PythonVersion {
    pub release: &'static str,
    tests: &'static str,
    pub numbers: [u32; 3],
    labels: &'static str,
    pub aliases: &'static [(&'static str, &'static str)],
    pub modules: &'static [(&'static str, &'static str, &'static str)],
}
include!(concat!(env!("OUT_DIR"), "/python_versions.rs"));

fn error(requested: &str) -> String {
    format!("Error: unsupported Python version '{requested}'. Supported releases: {}", VERSIONS.iter().map(|v| v.release).collect::<Vec<_>>().join(", "))
}

pub fn select(flag: Option<&str>, file: &str) -> Result<&'static PythonVersion, String> {
    // Read configuration only when neither higher-priority source supplies a value.
    let environment = env::var_os("LUMEN_PYTHON");
    let requested = if let Some(flag) = flag {
        Some(flag.to_string())
    } else if let Some(value) = environment {
        Some(value.to_string_lossy().into_owned())
    } else {
        let sidecar = format!("{file}.lumen.json");
        match fs::read_to_string(&sidecar) {
            Ok(text) => {
                let data: Value = serde_json::from_str(&text).map_err(|e| format!("Error: {sidecar}: {e}"))?;
                match data.get("python").and_then(|p| p.get("version")) {
                    Some(Value::String(version)) => Some(version.clone()),
                    Some(_) => return Err(format!("Error: {sidecar}: python.version must be a string; {}", error("configuration"))),
                    None => None,
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("Error: {sidecar}: {e}")),
        }
    };
    let Some(requested) = requested else { return Ok(VERSIONS.iter().max_by_key(|v| v.numbers).unwrap()) };
    let parts: Vec<_> = requested.split('.').collect();
    if !(2..=3).contains(&parts.len()) || parts.iter().any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) || (p.len() > 1 && p.starts_with('0'))) {
        return Err(error(&requested));
    }
    let numbers: Vec<u32> = parts[..2].iter().map(|p| p.parse()).collect::<Result<_, _>>().map_err(|_| error(&requested))?;
    let version = VERSIONS.iter().find(|v| v.numbers[..2] == numbers[..2]).ok_or_else(|| error(&requested))?;
    if parts.len() == 3 && requested != version.release {
        eprintln!("Warning: requested {requested}, running {} compatibility, tested against the CPython {} suite", version.release, version.release);
    }
    Ok(version)
}

impl PythonVersion {
    pub fn definition(&self, base: &str) -> Result<String, String> {
        let mut data: Value = serde_json::from_str(base).map_err(|e| e.to_string())?;
        let labels: Value = serde_json::from_str(self.labels).map_err(|e| e.to_string())?;
        let object = data.as_object_mut().ok_or("Python definition must be an object")?;
        for (key, value) in labels.as_object().ok_or("Python overlay must be an object")? {
            object.insert(key.clone(), value.clone());
        }
        serde_json::to_string(&data).map_err(|e| e.to_string())
    }

    pub fn module_source(&self, name: &str, source: &str) -> String {
        // The shared sources remain the 3.14 library. Identity follows the pin,
        // including when a later version supplies its own source overlay.
        let [major, minor, micro] = self.numbers;
        match name {
            "sys" => format!("{source}\nversion_info = ({major}, {minor}, {micro}, 'final', 0)\nversion = '{} (Lumen)'\nhexversion = {}\n", self.release, (major << 24) | (minor << 16) | (micro << 8) | 0xf0),
            // The test package's own search path names the reference suite
            // beside the library; it follows the selected release too.
            "test" => source.replace("/tests/python/test", &format!("/{}/test", self.tests)),
            "test.support" => {
                let components = self.tests.split('/').map(|part| format!("{part:?}")).collect::<Vec<_>>().join(", ");
                source.replace("'tests', 'python'", &components)
            }
            "doctest" | "_doctest_compat" => source.replace("/tests/python/test_", &format!("/{}/test_", self.tests)),
            "platform" => format!("{source}\ndef python_version():\n    return '{}'\n", self.release),
            _ => source.to_string(),
        }
    }
}
