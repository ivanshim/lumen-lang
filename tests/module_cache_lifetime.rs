use std::{fs, process::Command};

#[test]
fn retired_module_cache_does_not_keep_globals_alive() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("probe").join(format!("module-lifetime-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("cached_owner.py"), r#"
class Payload:
    pass
payload = Payload()
def read():
    return len([]), bytes
globals()['len'] = lambda value: 7
globals()['bytes'] = payload
"#).unwrap();
    let script = directory.join("check.py");
    fs::write(&script, r#"
import gc
import sys
import weakref
sys.path.insert(0, __file__.rsplit('/', 1)[0])
for replace in (False, True):
    import cached_owner
    module_ref = weakref.ref(cached_owner)
    payload_ref = weakref.ref(cached_owner.payload)
    assert cached_owner.read() == (7, cached_owner.payload)
    if replace:
        sys.modules['cached_owner'] = type(sys)('cached_owner')
    else:
        del sys.modules['cached_owner']
    del cached_owner
    gc.collect()
    assert module_ref() is None, 'cached module survived collection'
    assert payload_ref() is None, 'cached globals survived collection'
    sys.modules.pop('cached_owner', None)
"#).unwrap();
    for kernel in ["stack8", "microcode7"] {
        let output = Command::new(env!("CARGO_BIN_EXE_lumen-lang"))
            .args(["--kernel", kernel]).arg(&script).output().unwrap();
        assert!(output.status.success(), "{kernel}: {}", String::from_utf8_lossy(&output.stderr));
    }
    fs::remove_dir_all(directory).unwrap();
}
