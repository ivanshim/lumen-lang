# TCP server primitives for the native socket bridge.
# The public http.server module remains the unchanged CPython v3.14.8 copy.
import io
import select
import socket
import sys
import threading
import time
import traceback


class BaseServer:
    # Store the address and handler; concrete transports provide the socket.
    def __init__(self, server_address, RequestHandlerClass):
        self.server_address = server_address
        self.RequestHandlerClass = RequestHandlerClass
        self.__shutdown_request = False
        self.__is_shut_down = threading.Event()

    # Process ready requests until another thread requests shutdown.
    def serve_forever(self, poll_interval=0.5):
        self.__is_shut_down.clear()
        try:
            while not self.__shutdown_request:
                if select.select([self], [], [], poll_interval)[0]:
                    if self.__shutdown_request:
                        break
                    self._handle_request_noblock()
                self.service_actions()
        finally:
            self.__shutdown_request = False
            self.__is_shut_down.set()

    # Wait for serve_forever to return; callers must use another thread.
    def shutdown(self):
        self.__shutdown_request = True
        self.__is_shut_down.wait()

    # Leave periodic transport-specific maintenance to subclasses.
    def service_actions(self):
        pass

    # Wait for one request, respecting the socket and server timeouts.
    def handle_request(self):
        timeout = self.socket.gettimeout()
        if timeout is None:
            timeout = self.timeout
        elif self.timeout is not None:
            timeout = min(timeout, self.timeout)
        deadline = None if timeout is None else time.monotonic() + timeout
        while True:
            if select.select([self], [], [], timeout)[0]:
                return self._handle_request_noblock()
            if deadline is not None:
                timeout = deadline - time.monotonic()
                if timeout <= 0:
                    return self.handle_timeout()

    # Give subclasses a hook for idle request timeouts.
    def handle_timeout(self):
        pass

    # Accept, validate, and dispatch one request with cleanup on every path.
    def _handle_request_noblock(self):
        try:
            request, client_address = self.get_request()
        except OSError:
            return
        if not self.verify_request(request, client_address):
            self.shutdown_request(request)
            return
        try:
            self.process_request(request, client_address)
        except Exception:
            try:
                self.handle_error(request, client_address)
            finally:
                self.shutdown_request(request)
        except BaseException:
            self.shutdown_request(request)
            raise

    # Accept requests unless a subclass supplies an access policy.
    def verify_request(self, request, client_address):
        return True

    # Construct the handler and close its accepted connection.
    def process_request(self, request, client_address):
        try:
            self.finish_request(request, client_address)
        finally:
            self.shutdown_request(request)

    # Let the handler manage setup, request handling, and finalization.
    def finish_request(self, request, client_address):
        self.RequestHandlerClass(request, client_address, self)

    # Report handler failures while allowing the server to keep serving.
    def handle_error(self, request, client_address):
        print("Exception occurred during processing of request from", client_address,
              file=sys.stderr)
        traceback.print_exc()

    # Return the server for a context-managed lifetime.
    def __enter__(self):
        return self

    # Close the transport when the context ends.
    def __exit__(self, *args):
        self.server_close()


class TCPServer(BaseServer):
    address_family = socket.AF_INET
    socket_type = socket.SOCK_STREAM
    request_queue_size = 5
    allow_reuse_address = False
    allow_reuse_port = False
    timeout = None

    # Create and optionally bind the listening socket, closing on failure.
    def __init__(self, server_address, RequestHandlerClass, bind_and_activate=True):
        super().__init__(server_address, RequestHandlerClass)
        self.socket = socket.socket(self.address_family, self.socket_type)
        if bind_and_activate:
            try:
                self.server_bind()
                self.server_activate()
            except BaseException:
                self.server_close()
                raise

    # Bind the configured address and retain the assigned ephemeral port.
    def server_bind(self):
        if self.allow_reuse_address and hasattr(socket, "SO_REUSEADDR"):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        if self.allow_reuse_port and hasattr(socket, "SO_REUSEPORT"):
            self.socket.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEPORT, 1)
        self.socket.bind(self.server_address)
        self.server_address = self.socket.getsockname()

    # Start listening after binding succeeds.
    def server_activate(self):
        self.socket.listen(self.request_queue_size)

    # Release the listening descriptor.
    def server_close(self):
        self.socket.close()

    # Expose the descriptor to select and selectors.
    def fileno(self):
        return self.socket.fileno()

    # Accept one real connection from the host.
    def get_request(self):
        return self.socket.accept()

    # Finish the write side and close even if shutdown fails.
    def shutdown_request(self, request):
        try:
            request.shutdown(socket.SHUT_WR)
        except OSError:
            pass
        finally:
            self.close_request(request)

    # Release an accepted connection.
    def close_request(self, request):
        request.close()


class BaseRequestHandler:
    # Run setup before handling, and always finalize a successfully set-up handler.
    def __init__(self, request, client_address, server):
        self.request = request
        self.client_address = client_address
        self.server = server
        self.setup()
        try:
            self.handle()
        finally:
            self.finish()

    # Allow subclasses to initialize per-request state.
    def setup(self):
        pass

    # Allow subclasses to process the request.
    def handle(self):
        pass

    # Allow subclasses to release per-request state.
    def finish(self):
        pass


class _SocketWriter(io.BufferedIOBase):
    # Retain the socket without acquiring a second descriptor.
    def __init__(self, sock):
        self._sock = sock

    # Advertise the write-only stream interface.
    def writable(self):
        return True

    # Send the complete buffer and report its byte length.
    def write(self, data):
        self._sock.sendall(data)
        return memoryview(data).nbytes

    # Expose the underlying descriptor.
    def fileno(self):
        return self._sock.fileno()


class StreamRequestHandler(BaseRequestHandler):
    rbufsize = -1
    wbufsize = 0
    timeout = None
    disable_nagle_algorithm = False

    # Wrap the accepted socket with the streams used by HTTP handlers.
    def setup(self):
        self.connection = self.request
        if self.timeout is not None:
            self.connection.settimeout(self.timeout)
        if self.disable_nagle_algorithm:
            self.connection.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, True)
        self.rfile = self.connection.makefile("rb", self.rbufsize)
        self.wfile = (_SocketWriter(self.connection) if self.wbufsize == 0
                      else self.connection.makefile("wb", self.wbufsize))

    # Flush output and close both stream wrappers on every path.
    def finish(self):
        try:
            if not self.wfile.closed:
                try:
                    self.wfile.flush()
                except OSError:
                    pass
        finally:
            try:
                self.wfile.close()
            finally:
                self.rfile.close()


class ThreadingMixIn:
    daemon_threads = False
    block_on_close = True

    # Run one handler in its worker and close the connection afterwards.
    def process_request_thread(self, request, client_address):
        try:
            self.finish_request(request, client_address)
        except Exception:
            self.handle_error(request, client_address)
        finally:
            self.shutdown_request(request)

    # Start a native worker; unsupported threading raises instead of faking work.
    def process_request(self, request, client_address):
        worker = threading.Thread(target=self.process_request_thread,
                                  args=(request, client_address))
        worker.daemon = self.daemon_threads
        worker.start()
        if not self.daemon_threads:
            if not hasattr(self, "_workers"):
                self._workers = []
            self._workers.append(worker)

    # Close the listener and wait for non-daemon handlers when requested.
    def server_close(self):
        super().server_close()
        if self.block_on_close:
            for worker in getattr(self, "_workers", ()):
                worker.join()
