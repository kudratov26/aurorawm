.PHONY: all build release install clean uninstall test fmt clippy

# Build targets
all: build

build:
	cargo build

release:
	cargo build --release

# Installation
install: release
	@echo "Installing AuroraWM..."
	install -Dm755 target/release/aurorawm $(DESTDIR)/usr/bin/aurorawm
	install -Dm755 aurorawm-session $(DESTDIR)/usr/bin/aurorawm-session
	install -Dm644 aurorawm.desktop $(DESTDIR)/usr/share/wayland-sessions/aurorawm.desktop
	install -Dm644 aurorawm.service $(DESTDIR)/usr/lib/systemd/system/aurorawm.service
	@echo "Installation complete!"

uninstall:
	@echo "Uninstalling AuroraWM..."
	rm -f $(DESTDIR)/usr/bin/aurorawm
	rm -f $(DESTDIR)/usr/share/wayland-sessions/aurorawm.desktop
	rm -f $(DESTDIR)/usr/lib/systemd/system/aurorawm.service
	@echo "Uninstallation complete!"

# Development
clean:
	cargo clean

test:
	cargo test

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

# Run
run:
	cargo run

run-release:
	cargo run --release

# Dependencies check
deps:
	@echo "Checking dependencies..."
	@command -v cargo >/dev/null 2>&1 || { echo "cargo not found"; exit 1; }
	@command -v wayland-scanner >/dev/null 2>&1 || { echo "wayland-scanner not found"; exit 1; }
	@echo "All dependencies found!"
