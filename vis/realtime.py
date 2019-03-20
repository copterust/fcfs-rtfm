import serial
import struct
import visdom
import numpy as np

#port = serial.Serial("/dev/ttyUSB0", baudrate=115200, timeout=3.0)
viz = visdom.Visdom(port=8097, server="http://localhost")
#viz.line(Y=np.random.rand(10),
#         win="Yaw",
#         name="Yaw")

xs = []
ys = []
x = 0

while True:
#    rcv = port.read(10)
#    print("Got bytes: {}".format(rcv))
#    value = struct.unpack('d', rcv)[0]
#    print("Value is: {}".format(value))
    xs.append(x)
    x = x + 1
    ys.append(np.random.rand(1))
    viz.line(X=xs,
             Y=ys,
             win="Yaw",
             name="Yaw",
             update='append')
