import serial
import struct
import visdom
import numpy as np

port = serial.Serial("/dev/ttyUSB0", baudrate=460800, timeout=3.0)
viz = visdom.Visdom(port=8097, server="http://localhost")
viz.line(X=np.array([0]),
         Y=np.array([0]),
         win="Yaw",
         name="Yaw",
         update='append')
x = 0
while True:
    #rcv = port.read(4)
    #print("Got bytes: {}".format(rcv))
    #value = struct.unpack('d', rcv)[0]
    #print("Value is: {}".format(value))
    x = x + 1
    viz.line(X=np.array([x]),
             Y=np.random.rand(1),
             win="Yaw",
             name="Yaw",
             update='append')
