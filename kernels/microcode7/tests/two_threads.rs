//! Two interpreter threads at once: each one's cleanups (a finalizer, a
//! generator's close) must run at the step they are due, whatever the
//! other thread is doing with its own queues.
const PYTHON: &str = include_str!("../../../langs/python.json");

const PROGRAM: &str = r#"
log = []
class D:
    def __del__(self):
        log.append('del')
def gen():
    try:
        yield 1
    finally:
        log.append('closed')
for round in range(60):
    log.clear()
    d = D()
    d = None
    assert log == ['del'], ('finalizer late', round, log)
    log.clear()
    g = gen()
    next(g)
    g = None
    assert log == ['closed'], ('generator close late', round, log)
"#;

#[test]
fn cleanups_stay_with_their_own_thread() {
    let workers: Vec<_> = (0..2).map(|_| {
        std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
            lumen_microcode7::run_definition(PYTHON, PROGRAM, &[], &[])
        }).unwrap()
    }).collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), Ok(()));
    }
}
