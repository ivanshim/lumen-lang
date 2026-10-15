# Native terminal-control adapter for CPython v3.14.8 Modules/termios.c.
# Copyright (c) Python Software Foundation; PSF License in tests/python-3.14.8/LICENSE.
from posix import _call, _cint


class error(Exception):
    pass


# Accept an int or any object with a fileno() method, as CPython's fd converter does.
def _descriptor(fd):
    if type(fd) is int:
        return _cint(fd)
    try:
        method = fd.fileno
    except AttributeError:
        raise TypeError('argument must be an int, or have a fileno() method.') from None
    number = method()
    if type(number) is not int:
        raise TypeError('fileno() returned a non-integer')
    return _cint(number)


# Run one host call, turning an OS failure into termios.error(errno, message).
def _run(op, *args):
    try:
        return _call(op, *args)
    except OSError as exc:
        raise error(exc.errno, exc.strerror) from None


# Read a terminal's mode as [iflag, oflag, cflag, lflag, ispeed, ospeed, cc].
def tcgetattr(fd, /):
    iflag, oflag, cflag, lflag, ispeed, ospeed, controls = _run('tcgetattr', _descriptor(fd))
    cc = [controls[i:i + 1] for i in range(len(controls))]
    if not lflag & ICANON:
        cc[VMIN] = controls[VMIN]
        cc[VTIME] = controls[VTIME]
    return [iflag, oflag, cflag, lflag, ispeed, ospeed, cc]


# Apply a mode in the shape tcgetattr returns, at the moment `when` names.
def tcsetattr(fd, when, attributes, /):
    descriptor = _descriptor(fd)
    when = _cint(when)
    _run('tcgetattr', descriptor)  # C reads the current mode first, so a non-terminal fails before the arguments are checked
    if not isinstance(attributes, list) or len(attributes) != 7:
        raise TypeError('tcsetattr, arg 3: must be 7 element list')
    cc = attributes[6]
    if not isinstance(cc, list) or len(cc) != NCCS:
        raise TypeError('tcsetattr: attributes[6] must be %d element list' % NCCS)
    controls = bytearray()
    for item in cc:
        if isinstance(item, bytes) and len(item) == 1:
            controls += item
        elif type(item) is int:
            controls.append(item & 255)
        else:
            raise TypeError('tcsetattr: elements of attributes must be characters or integers')
    iflag, oflag, cflag, lflag, ispeed, ospeed = (_cint(attributes[i]) for i in range(6))
    _run('tcsetattr', descriptor, when, iflag, oflag, cflag, lflag, ispeed, ospeed, bytes(controls))


# Send a break for the given duration.
def tcsendbreak(fd, duration, /):
    _run('tcflow', _descriptor(fd), 0, _cint(duration))


# Wait until all output written to the terminal has been sent.
def tcdrain(fd, /):
    _run('tcflow', _descriptor(fd), 1, 0)


# Discard queued input and/or output.
def tcflush(fd, queue, /):
    _run('tcflow', _descriptor(fd), 2, _cint(queue))


# Suspend or resume input or output on the terminal.
def tcflow(fd, action, /):
    _run('tcflow', _descriptor(fd), 3, _cint(action))


B0 = 0
B1000000 = 4104
B110 = 3
B115200 = 4098
B1152000 = 4105
B1200 = 9
B134 = 4
B150 = 5
B1500000 = 4106
B1800 = 10
B19200 = 14
B200 = 6
B2000000 = 4107
B230400 = 4099
B2400 = 11
B2500000 = 4108
B300 = 7
B3000000 = 4109
B3500000 = 4110
B38400 = 15
B4000000 = 4111
B460800 = 4100
B4800 = 12
B50 = 1
B500000 = 4101
B57600 = 4097
B576000 = 4102
B600 = 8
B75 = 2
B921600 = 4103
B9600 = 13
BRKINT = 2
BS0 = 0
BS1 = 8192
BSDLY = 8192
CBAUD = 4111
CBAUDEX = 4096
CDSUSP = 25
CEOF = 4
CEOL = 0
CEOT = 4
CERASE = 127
CFLUSH = 15
CIBAUD = 269418496
CINTR = 3
CKILL = 21
CLNEXT = 22
CLOCAL = 2048
CQUIT = 28
CR0 = 0
CR1 = 512
CR2 = 1024
CR3 = 1536
CRDLY = 1536
CREAD = 128
CRPRNT = 18
CRTSCTS = 2147483648
CS5 = 0
CS6 = 16
CS7 = 32
CS8 = 48
CSIZE = 48
CSTART = 17
CSTOP = 19
CSTOPB = 64
CSUSP = 26
CWERASE = 23
ECHO = 8
ECHOCTL = 512
ECHOE = 16
ECHOK = 32
ECHOKE = 2048
ECHONL = 64
ECHOPRT = 1024
EXTA = 14
EXTB = 15
FF0 = 0
FF1 = 32768
FFDLY = 32768
FIOASYNC = 21586
FIOCLEX = 21585
FIONBIO = 21537
FIONCLEX = 21584
FIONREAD = 21531
FLUSHO = 4096
HUPCL = 1024
ICANON = 2
ICRNL = 256
IEXTEN = 32768
IGNBRK = 1
IGNCR = 128
IGNPAR = 4
IMAXBEL = 8192
INLCR = 64
INPCK = 16
IOCSIZE_MASK = 1073676288
IOCSIZE_SHIFT = 16
ISIG = 1
ISTRIP = 32
IUCLC = 512
IXANY = 2048
IXOFF = 4096
IXON = 1024
NCC = 8
NCCS = 32
NL0 = 0
NL1 = 256
NLDLY = 256
NOFLSH = 128
N_MOUSE = 2
N_PPP = 3
N_SLIP = 1
N_STRIP = 4
N_TTY = 0
OCRNL = 8
OFDEL = 128
OFILL = 64
OLCUC = 2
ONLCR = 4
ONLRET = 32
ONOCR = 16
OPOST = 1
PARENB = 256
PARMRK = 8
PARODD = 512
PENDIN = 16384
TAB0 = 0
TAB1 = 2048
TAB2 = 4096
TAB3 = 6144
TABDLY = 6144
TCFLSH = 21515
TCGETA = 21509
TCGETS = 21505
TCIFLUSH = 0
TCIOFF = 2
TCIOFLUSH = 2
TCION = 3
TCOFLUSH = 1
TCOOFF = 0
TCOON = 1
TCSADRAIN = 1
TCSAFLUSH = 2
TCSANOW = 0
TCSBRK = 21513
TCSBRKP = 21541
TCSETA = 21510
TCSETAF = 21512
TCSETAW = 21511
TCSETS = 21506
TCSETSF = 21508
TCSETSW = 21507
TCXONC = 21514
TIOCCONS = 21533
TIOCEXCL = 21516
TIOCGETD = 21540
TIOCGICOUNT = 21597
TIOCGLCKTRMIOS = 21590
TIOCGPGRP = 21519
TIOCGSERIAL = 21534
TIOCGSOFTCAR = 21529
TIOCGWINSZ = 21523
TIOCINQ = 21531
TIOCLINUX = 21532
TIOCMBIC = 21527
TIOCMBIS = 21526
TIOCMGET = 21525
TIOCMIWAIT = 21596
TIOCMSET = 21528
TIOCM_CAR = 64
TIOCM_CD = 64
TIOCM_CTS = 32
TIOCM_DSR = 256
TIOCM_DTR = 2
TIOCM_LE = 1
TIOCM_RI = 128
TIOCM_RNG = 128
TIOCM_RTS = 4
TIOCM_SR = 16
TIOCM_ST = 8
TIOCNOTTY = 21538
TIOCNXCL = 21517
TIOCOUTQ = 21521
TIOCPKT = 21536
TIOCPKT_DATA = 0
TIOCPKT_DOSTOP = 32
TIOCPKT_FLUSHREAD = 1
TIOCPKT_FLUSHWRITE = 2
TIOCPKT_NOSTOP = 16
TIOCPKT_START = 8
TIOCPKT_STOP = 4
TIOCSCTTY = 21518
TIOCSERCONFIG = 21587
TIOCSERGETLSR = 21593
TIOCSERGETMULTI = 21594
TIOCSERGSTRUCT = 21592
TIOCSERGWILD = 21588
TIOCSERSETMULTI = 21595
TIOCSERSWILD = 21589
TIOCSER_TEMT = 1
TIOCSETD = 21539
TIOCSLCKTRMIOS = 21591
TIOCSPGRP = 21520
TIOCSSERIAL = 21535
TIOCSSOFTCAR = 21530
TIOCSTI = 21522
TIOCSWINSZ = 21524
TOSTOP = 256
VDISCARD = 13
VEOF = 4
VEOL = 11
VEOL2 = 16
VERASE = 2
VINTR = 0
VKILL = 3
VLNEXT = 15
VMIN = 6
VQUIT = 1
VREPRINT = 12
VSTART = 8
VSTOP = 9
VSUSP = 10
VSWTC = 7
VSWTCH = 7
VT0 = 0
VT1 = 16384
VTDLY = 16384
VTIME = 5
VWERASE = 14
XCASE = 4
XTABS = 6144
