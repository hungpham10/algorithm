.PHONY: setup lint build test clean all server proxy client up

CARGO := cargo
TRUNK := trunk
PYTHON := python3

BACKEND_DIR := pkgs/backend
SERVICES_DIR := pkgs/services
PROXY_DIR := pkgs/proxy
FRONTEND_DIR := pkgs/frontend

up:
	@docker-compose up --build

setup:
	@if ! command -v rustc > /dev/null; then \
		echo "Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
	fi
	@rustup component add clippy rustfmt
	@if ! command -v trunk > /dev/null; then \
		$(CARGO) install trunk; \
	fi

lint:
	@echo "Running Clippy & Fmt..."
	$(CARGO) clippy --all-targets --all-features -- -D warnings
	$(CARGO) fmt --all -- --check

proxy:
	@echo "Building Proxy (Release)..."
	$(CARGO) build -p proxy --release

server:
	@echo "Building Services/Server (Release)..."
	$(CARGO) build -p services --release

client:
	@echo "Building Frontend (Trunk Release)..."
	cd $(FRONTEND_DIR) && $(TRUNK) build --release

test-algorithm:
	$(CARGO) test -p algorithm

test-integration:
	$(CARGO) test -p integration

test-services:
	$(CARGO) test -p services

test: test-services test-algorithm test-integration

all: lint test server proxy client

clean:
	@echo "Cleaning workspace..."
	$(CARGO) clean
	@rm -rf dist/
	@find . -type d -name "__pycache__" -exec rm -rf {} +
	@find . -type f -name "*.pyc" -delete
