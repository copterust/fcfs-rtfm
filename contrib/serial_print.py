#!python
import serial
import sys
import numpy as np

port = serial.Serial(sys.argv[1], baudrate=460800, timeout=3.0)

# fmt:
while True:
    line = port.read_until()
    try:
        comps = [float(a.strip()) for a in line.split(b';') if a.strip()]
        (ax, ay, az, gx, gy, gz, dts, y, p, r) = comps
        print(ax, ay, az, gx, gy, gz, dts, y, p, r)
    except Exception:
        pass
