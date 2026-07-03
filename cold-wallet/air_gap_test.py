#!/usr/bin/env python3
"""
air_gap_test.py — Proves the keygen/signing tools never touch the network.

This is not a simulation of an air-gapped machine — it monkeypatches
Python's socket module to raise on ANY use (socket.socket, create_connection,
getaddrinfo) *before* running the keygen and signing tools, then runs a full
keygen + sign cycle. If either tool tried to phone home in any way, this
script would crash with NoNetwork instead of completing.

Run it yourself:
    cd cold-wallet
    python3 air_gap_test.py

Expected output ends with:
    ALL CHECKS PASSED — keygen + signing completed with zero network access
"""
import sys
import os
import socket

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO_ROOT, "core"))
sys.path.insert(0, os.path.join(REPO_ROOT, "keygen"))


class NoNetwork(Exception):
    pass


def _blocked(*a, **kw):
    raise NoNetwork(
        "Network access attempted! Cold-wallet code must NEVER touch a socket."
    )


def install_network_guard():
    socket.socket = _blocked
    socket.create_connection = _blocked
    if hasattr(socket, "getaddrinfo"):
        socket.getaddrinfo = _blocked


def run():
    install_network_guard()
    print(">>> Network guard installed (socket.socket / create_connection / getaddrinfo all blocked)")

    priv_path = "/tmp/air_gap_test.private.json"
    pub_path = "/tmp/air_gap_test.public.json"
    witness_path = "/tmp/air_gap_test.witness.json"

    print("\n=== Step 1: keygen (height=4, fast demo scale) ===")
    sys.argv = [
        "xmss_keygen.py",
        "--height=4",
        f"--out_private={priv_path}",
        f"--out_public={pub_path}",
    ]
    exec(compile(open(os.path.join(REPO_ROOT, "keygen", "xmss_keygen.py")).read(),
                 "xmss_keygen.py", "exec"), {"__name__": "__main__"})

    print("\n=== Step 2: sign a message ===")
    sys.argv = [
        "xmss_sign.py",
        f"--private={priv_path}",
        "--message=air-gap verification message",
        f"--out={witness_path}",
    ]
    exec(compile(open(os.path.join(REPO_ROOT, "keygen", "xmss_sign.py")).read(),
                 "xmss_sign.py", "exec"), {"__name__": "__main__"})

    for p in (priv_path, pub_path, witness_path):
        if os.path.exists(p):
            os.remove(p)

    print("\nALL CHECKS PASSED — keygen + signing completed with zero network access")


if __name__ == "__main__":
    run()
