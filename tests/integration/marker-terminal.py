#!/usr/bin/env python3
"""Disposable input receiver. Never evaluates input or stores non-marker lines."""
import argparse
import json
import os
from pathlib import Path
import select
import sys
import time

MARKER='depthos-02d-ok'

def matches(line):
    return line.rstrip('\r\n') == MARKER


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pid-file',type=Path,required=True)
    parser.add_argument('--ack-file',type=Path,required=True)
    args=parser.parse_args()
    args.pid_file.write_text(str(os.getpid()))
    print('DEPTHOS POC-02D — DISPOSABLE INPUT TEST',flush=True)
    print('This is NOT a shell. Type depthos-02d-ok then Enter after dismissing DepthOS.',flush=True)
    start=time.monotonic()
    while time.monotonic()-start < 900:
        ready,_,_=select.select([sys.stdin],[],[],.25)
        if not ready:continue
        line=sys.stdin.readline(256)
        if not line:break
        if matches(line):
            args.ack_file.write_text(json.dumps({'matched':True,'marker':MARKER,'wallTime':time.time()})+'\n')
            print('INPUT VERIFIED — keyboard delivery reached this test terminal.',flush=True)
        del line

if __name__=='__main__':main()
