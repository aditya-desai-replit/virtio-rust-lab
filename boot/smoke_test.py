import os
from pathlib import Path
import selectors
import signal
import subprocess
import time

BOOT = Path(__file__).resolve().parent
CACHE = BOOT.parent / ".cache" / "boot"


def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    process = subprocess.Popen(
        ["bash", str(BOOT / "run.sh")],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=True,
    )
    selector = selectors.DefaultSelector()
    selector.register(process.stdout, selectors.EVENT_READ)
    output = bytearray()

    def collect(seconds, until=None):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline and selector.get_map():
            for key, _ in selector.select(0.02):
                data = os.read(key.fd, 65536)
                if not data:
                    selector.unregister(key.fileobj)
                    return
                output.extend(data)
            if until and until in output:
                return

    def send(command):
        # Pace input to avoid overrunning the emulated UART's receive FIFO.
        for byte in command.encode():
            process.stdin.write(bytes([byte]))
            process.stdin.flush()
            collect(0.03)

    try:
        collect(20, b"/ # ")
        if b"/ # " not in output:
            raise RuntimeError("Guest shell did not appear")
        send("uname -r\n")
        collect(1)
        send("cat /proc/cmdline\n")
        collect(1)
        send("echo VM_OK\n")
        collect(1)
        normalized = output.replace(b"\r\n", b"\n")
        if b"\n5.4.81\n" not in normalized or b"\nVM_OK\n" not in normalized:
            raise RuntimeError("Expected guest command output not observed")
        send("reboot -f\n")
        collect(10)
        if process.wait(timeout=5) != 0:
            raise RuntimeError("VMM exited unsuccessfully")
        print("PASS: Linux booted, guest commands executed, and VMM exited.")
    finally:
        if process.poll() is None:
            os.killpg(process.pid, signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait(timeout=5)
        selector.close()
        process.stdin.close()
        process.stdout.close()
        (CACHE / "boot.log").write_bytes(output)
        print(output.decode(errors="replace")[-1800:])


if __name__ == "__main__":
    main()