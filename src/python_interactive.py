# The CLI's interactive input loop. Compilation state and the user namespace
# survive each command; this is also used after a script or -c with -i.
def _lumen_interact(source, filename, namespace, quiet=False, version="3.14.8"):
    import codeop
    import sys
    if not quiet:
        sys.stderr.write("Python " + version + " (lumen-lang)\n")
        sys.stderr.flush()
    compiler = codeop.CommandCompiler()
    def report():
        import traceback
        traceback.print_exc()
    if source:
        try:
            initial = compile(source, filename, 'exec')
            import __future__
            for name in __future__.all_feature_names:
                compiler.compiler.flags |= initial.co_flags & getattr(__future__, name).compiler_flag
            exec(initial, namespace)
        except Exception:
            report()
    if not hasattr(sys, 'ps1'):
        sys.ps1 = '>>> '
    if not hasattr(sys, 'ps2'):
        sys.ps2 = '... '
    pending = []
    while True:
        try:
            sys.stderr.write(sys.ps2 if pending else sys.ps1)
            sys.stderr.flush()
            line = sys.stdin.readline()
            if not line:
                if pending:
                    # At EOF, finish the buffered input without incomplete-input flags.
                    command = compiler.compiler('\n'.join(pending), '<stdin>', 'single', incomplete_input=False)
                    pending = []
                    exec(command, namespace)
                sys.stderr.write("\n")
                sys.stderr.flush()
                return
            pending.append(line.rstrip('\n'))
            command = compiler('\n'.join(pending), '<stdin>', 'single')
            if command is None:
                continue
            pending = []
            exec(command, namespace)
        except KeyboardInterrupt:
            pending = []
            sys.stderr.write('\nKeyboardInterrupt\n')
        except Exception:
            pending = []
            report()
