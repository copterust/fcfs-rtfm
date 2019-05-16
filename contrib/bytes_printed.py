#!python
import serial
import struct
import sys
import numpy as np

port = serial.Serial(sys.argv[1], baudrate=460800, timeout=3.0)

MAGIC = b'lol'
PACKET_BYTES = 10 * 4
want_now = 68
raw = []

def fmt_and_reset(b):
    in_bytes = bytes(b)
    ten_floats = struct.unpack('ffffffffff', in_bytes)
    b.clear()
    return ten_floats

while True:
    packet = port.read(want_now)
    if not packet:
        pass
    parts = packet.split(MAGIC)
    skipped_part = False
    for p in parts:
        if (len(raw) + len(p) == PACKET_BYTES) and not skipped_part :
            raw.extend(p)
            tf = fmt_and_reset(raw)
            print(*tf)
        else:
            skipped_part = True
