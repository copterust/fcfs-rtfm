import serial
import struct
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.animation as animation

port = serial.Serial("/dev/ttyUSB0", baudrate=460800, timeout=3.0)

fig = plt.figure()
ax = fig.add_subplot(1, 1, 1)
xs = []
ys = []

def animate(i, xs, ys):
    rcv = port.read(4)
    value = struct.unpack('f', rcv)[0]
    value = np.degrees(value)

    xs.append(i)
    ys.append(value)

    xs = xs[-20:]
    ys = ys[-20:]

    ax.clear()
    ax.plot(xs, ys)

    plt.subplots_adjust(bottom=0.30)
    plt.title('Pitch Over Time')
    plt.ylabel('Pitch (deg)')


ani = animation.FuncAnimation(fig, animate, fargs=(xs, ys), interval=0.2)
plt.show()
