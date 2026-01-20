#!/usr/bin/env python3
"""
Yocto-based CI test script for StarryOS.
This script runs QEMU with the starry-minimal-image and verifies successful boot.
"""

import argparse
import datetime
import os
import subprocess
import sys
import time
import signal

def run_qemu_test(timeout_seconds=120):
    """Run QEMU test and check for successful boot."""
    
    # Start QEMU with runqemu
    qemu_cmd = ["runqemu", "starry-minimal-image", "nographic", "slirp"]
    
    print(f"Starting QEMU: {' '.join(qemu_cmd)}")
    
    try:
        p = subprocess.Popen(
            qemu_cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
    except FileNotFoundError:
        print("❌ runqemu command not found. Make sure Yocto environment is sourced.")
        return False
    
    # Expected prompt patterns
    BOOT_SUCCESS_PATTERNS = [
        "starry:~#",      # BusyBox shell prompt
        "login:",         # Login prompt
        "Welcome to",     # Welcome message
    ]
    
    buffer = ""
    start_time = datetime.datetime.now()
    boot_success = False
    
    try:
        while True:
            # Check timeout
            elapsed = (datetime.datetime.now() - start_time).total_seconds()
            if elapsed > timeout_seconds:
                print(f"\n❌ Timeout after {timeout_seconds} seconds")
                break
            
            # Check if process is still running
            if p.poll() is not None:
                print(f"\n❌ QEMU exited with code {p.returncode}")
                break
            
            # Read output (non-blocking would be better, but this works for simple cases)
            try:
                # Set a short timeout for reading
                import select
                if select.select([p.stdout], [], [], 1.0)[0]:
                    line = p.stdout.readline()
                    if line:
                        print(line, end="")
                        buffer += line
                        
                        # Check for success patterns
                        for pattern in BOOT_SUCCESS_PATTERNS:
                            if pattern in buffer:
                                boot_success = True
                                print(f"\n\033[32m✔ Found boot success pattern: '{pattern}'\033[0m")
                                break
                        
                        if boot_success:
                            break
            except Exception as e:
                print(f"Read error: {e}")
                time.sleep(0.1)
        
    except KeyboardInterrupt:
        print("\n⚠ Interrupted by user")
    finally:
        # Cleanup QEMU process
        if p.poll() is None:
            print("Terminating QEMU...")
            p.terminate()
            try:
                p.wait(timeout=5)
            except subprocess.TimeoutExpired:
                p.kill()
                p.wait()
    
    if boot_success:
        print("\n\033[32m✔ Boot into shell successful!\033[0m")
        return True
    else:
        print("\n\033[31m❌ Boot failed or timed out\033[0m")
        return False


def main():
    parser = argparse.ArgumentParser(description="StarryOS Yocto CI Test Script")
    parser.add_argument(
        "--timeout",
        type=int,
        default=120,
        help="Timeout in seconds for QEMU boot (default: 120)"
    )
    parser.add_argument(
        "--image",
        type=str,
        default="starry-minimal-image",
        help="Image name to test (default: starry-minimal-image)"
    )
    
    args = parser.parse_args()
    
    # Check if we're in a Yocto build environment
    if not os.environ.get("BUILDDIR"):
        print("⚠ Warning: BUILDDIR not set. Make sure to source oe-init-build-env first.")
    
    success = run_qemu_test(timeout_seconds=args.timeout)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
