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

def mk_points(f):
    return [{
            "measurement": "drone",
            "tags": {
            },
            "time": int(time.time()) * 1_000_000_000,
            "fields": f
        }]

def hacky(points):
    (ax, ay, az, gx, gy, gz, dts, y, p, r) = points
    d = dict(locals())
    del d['points']
    return d

while True:
    line = port.read_until()
    try:
        comps = [float(a.strip()) for a in line.split(b';') if a.strip()]
        client.write_points(mk_points(hacky(comps)))
    except Exception as e:
        print(e, file=sys.stderr)
        continue
