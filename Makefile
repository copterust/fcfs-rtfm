bin := fcfs-rtfm
NAME := $(bin)
release :=
MODE := $(if $(release),release,debug)
RELEASE_FLAG := $(if $(release),--release,)
target :=
TARGET := $(if $(target),"$(target)",thumbv7em-none-eabihf)
TARGET_PATH := ./target/$(TARGET)/$(MODE)
BIN := $(TARGET_PATH)/$(NAME)
fea :=
log := semihosting
level := info
configuration := dev
motors := quad
FEATURES := "--features=log_$(log),level_$(level),configuration_$(configuration),motors_$(motors),$(fea)"

$(BIN): build

memory:
	cp memory.$(configuration) memory.x

build: memory
	cargo -v build $(RELEASE_FLAG) --target $(TARGET) --bin $(NAME) --no-default-features $(FEATURES)

check:
	cargo -v check $(RELEASE_FLAG) --target $(TARGET) --bin $(NAME) --no-default-features $(FEATURES)

load: build
	sh -c "openocd & arm-none-eabi-gdb -q $(BIN) & wait"

gdb: build
	arm-none-eabi-gdb -q $(BIN)

clean:
	rm memory.x
	cargo -v clean

bloat:
	cargo -v bloat $(RELEASE_FLAG) --crates

details:
	cargo -v bloat $(RELEASE_FLAG) -n 100

.PHONY: build
