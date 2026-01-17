.PHONY: setup lint build test publish clean all

CARGO := cargo
TRUNK := trunk
PYTHON := python3
PIP_CACHE := .pip-cache

DIST_DIR := dist
TEST_DIR := tests
BACKEND_DIR := pkgs/backend

setup:
	$(PYTHON) -m pip install --upgrade pip
	$(PYTHON) -m pip install maturin twine pytest pyarrow patchelf --cache-dir $(PIP_CACHE)
	@if ! command -v rustc > /dev/null; then 													\
		echo "Installing Rust..."; 														\
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; 								\
		export PATH="$$HOME/.cargo/bin:$$PATH"; 												\
		$(CARGO) install trunk;															\
	fi
	@if [ -n "$(LIBTORCH_VERSION)" ]; then 														\
		curl -L https://download.pytorch.org/libtorch/cpu/libtorch-cxx11-abi-shared-with-deps-${LIBTORCH_VERSION}%2Bcpu.zip -o libtorch.zip 	\
    			&& unzip libtorch.zip -d /usr/local 												\
    			&& rm libtorch.zip;														\
	fi
lint:
	export PATH="$$HOME/.cargo/bin:$$PATH"  	&& 												\
	cd $(BACKEND_DIR) 				&& 												\
	rustup component add clippy rustfmt llvm-tools 	&& 												\
	$(CARGO) clippy --features python --lib		&& 												\
	$(CARGO) clippy 				&& 												\
	$(CARGO) fmt --all -- --check

library:
	@echo "Building release version $(VERSION)"
	@mkdir -p $(DIST_DIR)
	export PATH="$$HOME/.cargo/bin:$$PATH" &&													\
	cd $(BACKEND_DIR) && 																\
	if grep -q "^version" Cargo.toml; then 														\
		maturin build --release 														\
			--features python 														\
			--no-default-features 														\
			--bindings pyo3 														\
			--interpreter python3 														\
			--compatibility manylinux_2_17 													\
			--out dist; 															\
		cp dist/*.whl ../../$(DIST_DIR)/; 													\
	else 																		\
		echo "Missing version in Cargo.toml"; 													\
		exit 1; 																\
	fi
	@echo "Release wheel built in $(DIST_DIR)/"

proxy:
	@echo "Building release version $(VERSION)"
	@mkdir -p $(DIST_DIR)
	export PATH="$$HOME/.cargo/bin:$$PATH" &&													\
 	$(CARGO) build -p proxy --release --lib

server:
	@echo "Building release version $(VERSION)"
	@mkdir -p $(DIST_DIR)
	export PATH="$$HOME/.cargo/bin:$$PATH" &&													\
 	$(CARGO) build -p backend --release --bin algorithm

client:
	@echo "Building release version $(VERSION)"
	@mkdir -p $(DIST_DIR)
	export PATH="$$HOME/.cargo/bin:$$PATH" &&													\
	$(TRUNK) build -p frontend --release

ipython:
	@cp -av target/debug/libvnscope.dylib target/debug/vnscope.so || 										\
		cp -av target/release/libvnscope.dylib target/release/vnscope.so

install: library
	$(PYTHON) -m pip install --upgrade $(DIST_DIR)/*.whl

test-repl:
	$(PYTHON) -m pip install --upgrade $(DIST_DIR)/*.whl
	$(PYTHON) -m pytest -xvs $(TEST_DIR)/

test-backend:
	$(CARGO) test -p backend

test-proxy:
	$(CARGO) test -p proxy

test: library test-repl test-backend

all: test server client library

publish: library
	@echo "Publishing release version to PyPI"
	$(PYTHON) -m twine upload $(DIST_DIR)/*.whl

clean:
	rm -rf $(DIST_DIR)
	cd $(BACKEND_DIR) && cargo clean
	find . -type d -name "__pycache__" -exec rm -rf {} +
	find . -type d -name "*.egg-info" -exec rm -rf {} +
	find . -type f -name "*.pyc" -delete
