#!python
import serial
import sys
import numpy as np

port = serial.Serial(sys.argv[1], baudrate=460800, timeout=3.0)

# fmt:
# :<float>;\n
buff = []
while True:
    rcv = port.read(1)
    if rcv == b'\n':
        k = b''.join(buff)
        try:
            value = float(k.decode('ascii'))
            print(value)
        except Exception:
            # pyserial shit
            pass
        buff = []
    elif rcv == b':' or rcv == b';':
        pass
    else:
        buff.append(rcv)
