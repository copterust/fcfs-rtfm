bin := fcfs-rtfm
NAME := $(bin)
release :=
MODE := $(if $(release),release,debug)
RELEASE_FLAG := $(if $(release),--release,)
target :=
TARGET := $(if $(target),"$(target)",thumbv7em-none-eabihf)
TARGET_PATH := ./target/$(TARGET)/$(MODE)
BIN := $(TARGET_PATH)/$(NAME)

$(BIN): build

$(BIN).bin: $(BIN)
	arm-none-eabi-objcopy -S -O binary $(BIN) $(BIN).bin

build:
	cargo -v build $(RELEASE_FLAG) --target $(TARGET) --bin $(NAME) $(FEATURES)

flash: $(BIN).bin
	python2 ./loader/stm32loader.py -p $(TTY) -f F3 -e -w $(BIN).bin

load: build
	sh -c "openocd & arm-none-eabi-gdb -q $(BIN) & wait"

clean:
	cargo -v clean

.PHONY: build
