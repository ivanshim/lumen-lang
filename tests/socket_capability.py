#!/usr/bin/env python3
"""Check native socket capability gates and real single-threaded TCP/HTTP I/O."""
import socket
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from test import support
from test.support import threading_helper


# Check that one capability flag controls both public helper forms.
def check_capability():
    original = support.has_socket_support
    try:
        for available in (True, False):
            support.has_socket_support = available
            skipped = False
            try:
                support.requires_working_socket(module=True)
            except unittest.SkipTest:
                skipped = True
            assert skipped == (not available)

            @support.requires_working_socket()
            class Example(unittest.TestCase):
                # Supply a method so the class decorator has an ordinary testcase.
                def test_socket(self):
                    pass

            assert bool(getattr(Example, "__unittest_skip__", False)) == (not available)
    finally:
        support.has_socket_support = original
    assert original


# Exchange bytes through bind/listen/connect/accept without needing a worker thread.
def check_loopback():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        with socket.create_connection(listener.getsockname(), timeout=2) as client:
            accepted, peer = listener.accept()
            with accepted:
                accepted.settimeout(2)
                client.sendall(b"client")
                assert accepted.recv(6) == b"client"
                accepted.sendall(b"server")
                assert client.recv(6) == b"server"
                assert peer[0] == "127.0.0.1"


class Handler(BaseHTTPRequestHandler):
    # Send an ordinary response through the unchanged reference HTTP handler.
    def do_GET(self):
        body = b"real HTTP"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    # Suppress the access log in this assertion-based conformance check.
    def log_message(self, format, *args):
        pass


# Serve an actual HTTP request synchronously over the native TCP server.
def check_http():
    with HTTPServer(("127.0.0.1", 0), Handler) as server:
        server.timeout = 2
        with socket.create_connection(server.server_address, timeout=2) as client:
            client.sendall(b"GET / HTTP/1.0\r\nHost: localhost\r\n\r\n")
            server.handle_request()
            response = b""
            while True:
                data = client.recv(4096)
                if not data:
                    break
                response += data
            assert response.startswith(b"HTTP/1.0 200 ")
            assert response.endswith(b"\r\n\r\nreal HTTP")


# Refuse unavailable worker threads before asking any thread to start.
def check_thread_requirement():
    with threading_helper.start_threads([]):
        pass
    if not threading_helper.can_start_thread:
        class Worker:
            # Fail if the capability guard attempts to start an unsupported worker.
            def start(self):
                raise AssertionError("unsupported worker started")
        try:
            with threading_helper.start_threads([Worker()]):
                raise AssertionError("unsupported thread context entered")
        except unittest.SkipTest:
            pass


check_capability()
check_loopback()
check_http()
check_thread_requirement()
print("socket capability, loopback TCP, HTTP and thread requirements: OK")
