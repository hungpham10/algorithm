# Vietnamese Stock Data Mining

This project is for mining and analyzing Vietnamese stock data.

## Development

This project uses a Makefile to manage common development tasks such as setting up the environment, building components, and running tests.

### Prerequisites

- **Python 3**: Ensure you have Python 3 installed. You can download it from [python.org](https://www.python.org/).
- **Rust**: The backend components are written in Rust. If Rust is not installed, `make setup` or `make all` will attempt to install it for you using `rustup`. You can also install it manually from [rust-lang.org](https://www.rust-lang.org/).

### Common Makefile Targets

The following are some of the most common Makefile targets:

-   `make setup`: Installs all necessary Python and Rust dependencies. This includes installing Rust itself if not found.
-   `make build`: Compiles the Rust backend and builds the Python wheel.
-   `make test`: Runs the Python unit tests. This target depends on `build`, so it will build the project if necessary.
-   `make all`: A convenience target that runs `setup`, `build`, and `test` in sequence. This is the recommended target for a fresh start or to ensure everything is up-to-date and working.
-   `make lint`: Lints the Rust code using `cargo fmt` and `cargo clippy`.
-   `make clean`: Removes build artifacts, cached files, and virtual environments.

To use a target, navigate to the project's root directory in your terminal and run the desired `make` command (e.g., `make all`).

### Usage in Cloud Notebooks

You can run the Makefile targets in cloud-based notebook environments like Google Colab or Deepnote. To do this, prefix the Makefile command with an exclamation mark (`!`).

For example:
-   To set up, build, and test everything:
    ```bash
    !make all
    ```
-   To just run tests (assuming dependencies are already installed and the project is built):
    ```bash
    !make test
    ```
-   To install dependencies:
    ```bash
    !make setup
    ```

It is generally recommended to run `!make all` when starting a new session in a cloud notebook to ensure the environment is correctly prepared and all components are built and tested.
