#!python
import time
import math
import serial
import struct
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

buf = [0, 0, 0, 0]
i = 0
while True:
    rcv = port.read(1)
    if ord(rcv) == 0:
        k = b''.join(buf)
        value = struct.unpack('f', k)[0]
        print(value)
        if math.isfinite(value):
            client.write_points(mk_points(value))
        i = 0
    else:
        buf[i % 4] = rcv
        i += 1
