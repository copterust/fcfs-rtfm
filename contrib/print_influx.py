#!python
import time
import math
import serial
import sys
import numpy as np
from influxdb import InfluxDBClient


port = serial.Serial(sys.argv[1], baudrate=460800, timeout=3.0)

client = InfluxDBClient('localhost', 8086, 'root', 'root', 'example')
client.create_database('example')

def mk_points(value):
    return [{
            "measurement": "drone",
            "tags": {
            },
            "time": int(time.time()) * 1_000_000_000,
            "fields": {
                "rawpitch": value,
                "pitch": np.degrees(value)
            }
        }]


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
            if math.isfinite(value):
                client.write_points(mk_points(value))
        except Exception as e:
            # pyserial shit
            print(e, file=sys.stderr)
        buff = []
    elif rcv == b':' or rcv == b';':
        pass
    else:
        buff.append(rcv)
