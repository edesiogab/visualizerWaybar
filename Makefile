BINARY := waybar-audio-visualizer
PREFIX ?= $(HOME)/.local

.PHONY: build release run install uninstall clean

build:
	cargo build

release:
	cargo build --release

run:
	cargo run -- --interval-ms 100

install: release
	mkdir -p $(PREFIX)/bin
	cp target/release/$(BINARY) $(PREFIX)/bin/$(BINARY)
	chmod +x $(PREFIX)/bin/$(BINARY)
	@echo "Installed to $(PREFIX)/bin/$(BINARY)"

uninstall:
	rm -f $(PREFIX)/bin/$(BINARY)
	@echo "Removed $(PREFIX)/bin/$(BINARY)"

clean:
	cargo clean
