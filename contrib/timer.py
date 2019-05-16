#!python
import serial
import sys
import time

port = serial.Serial(sys.argv[1], baudrate=460800, timeout=10.0)

# fmt:
while True:
    before = time.time()
    line = port.read_until()
    t = time.time()
    try:
        comps = [float(a.strip()) for a in line.split(b';') if a.strip()]
        (ax, ay, az, gx, gy, gz, dts, y, p, r) = comps
        print('ourt=', t - before, 'devt=', dts)
    except Exception:
        pass
