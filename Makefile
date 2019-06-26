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
telemetry := bytes
configuration := dev
FEATURES := "--features=log_$(log),level_$(level),telemetry_$(telemetry),configuration_$(configuration),$(fea)"

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

boad: build
	bobbin -v load $(RELEASE_FLAG) --target $(TARGET) --bin $(NAME) $(FEATURES)

.PHONY: build
