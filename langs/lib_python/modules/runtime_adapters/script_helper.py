# Released process cleanup helper, layered over native interpreter argument routing.
def kill_python(p):
    """Run the given Popen process until completion and return stdout."""
    p.stdin.close()
    data = p.stdout.read()
    p.stdout.close()
    # try to cleanup the child so we don't appear to leak when running
    # with regrtest -R.
    p.wait()
    subprocess._cleanup()
    return data
