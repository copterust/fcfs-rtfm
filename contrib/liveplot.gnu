set terminal dumb 140, 44
plot "/tmp/data.txt" using 1:2 with lines title "e"
pause 4
reread
