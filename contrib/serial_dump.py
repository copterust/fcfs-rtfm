#!python
import serial
import struct
import sys
import numpy as np

port = serial.Serial(sys.argv[1], baudrate=460800, timeout=3.0)

buf = [0, 0, 0, 0]
i = 0
while True:
    rcv = port.read(1)
    if ord(rcv) == 0:
        k = b''.join(buf)
        print(i, buf, k, list(k))
        value = struct.unpack('f', k)[0]
        print(value)
        i = 0
    else:
        buf[i % 4] = rcv
        i += 1
