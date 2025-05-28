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

## CI/CD Pipeline

This project uses GitHub Actions to automate linting, testing, building, and publishing of the Python package. The workflow is defined in `.github/workflows/ci.yml` and primarily orchestrates Makefile targets.

### Triggers

The CI/CD pipeline is triggered on:
- Pushes to the `main` branch.
- Pull requests targeting the `main` branch (runs linters and tests via Makefile).
- Creation of new tags matching version patterns (e.g., `vX.Y.Z` or `X.Y.Z`).

### Workflow Overview

The GitHub Actions workflow uses Makefile targets to ensure consistency between local development and the CI environment.

1.  **Linting**:
    - The `lint` job in the workflow first runs `make setup` to install all necessary dependencies (including Rust and Python tools).
    - It then executes `make lint`, which internally runs `cargo fmt --check` and `cargo clippy` to check Rust code style and for common errors. This is automatically performed on pushes to `main` and pull requests.
2.  **Testing**:
    - The `test` job also starts by running `make setup` (though caching helps speed this up if dependencies haven't changed).
    - It then executes `make test`. This Makefile target handles building the Rust backend wheel (via `make build`) and then running Python unit tests (`pytest`). This is run on pushes to `main` and pull requests.
3.  **Development Builds (TestPyPI)**:
    - When changes are pushed to the `main` branch (and after the `test` job passes), the `build_publish_dev` job prepares a development version of the package.
    - The job first runs `make setup` to ensure build tools are available.
    - It then modifies `backend/Cargo.toml` to set a development version string: `X.Y.Z.dev0+<commit_sha>` (e.g., `0.1.0.dev0+a1b2c3d`), where `X.Y.Z` is the current version from `backend/Cargo.toml`.
    - After updating `Cargo.toml`, it runs `make build` to compile the package with this development version. The resulting wheel is placed in the `dist/` directory.
    - These development builds are automatically published from the `dist/` directory to [TestPyPI](https://test.pypi.org/).
4.  **Release Builds (PyPI)**:
    - When a new version tag (e.g., `v0.1.0` or `0.1.0`) is pushed to the repository, the `build_publish_release` job is triggered.
    - It begins by running `make setup`.
    - The pipeline then verifies that the pushed tag matches the version specified in `backend/Cargo.toml` (after removing any 'v' prefix from the tag).
    - If the versions match, `make build` is executed to compile the release version of the package (using the version in `backend/Cargo.toml` directly). The wheel is created in `dist/`.
    - This release version is automatically published from the `dist/` directory to the official [PyPI](https://pypi.org/).

### Making a Release

To make a new official release and publish it to PyPI:

1.  **Update Version in `Cargo.toml`**:
    Ensure the `version` field in `backend/Cargo.toml` is updated to the new desired release version (e.g., `version = "0.2.0"`).
2.  **Commit and Push**:
    Commit this change to `backend/Cargo.toml` and push it to the `main` branch. Ensure all tests are passing on `main` (verified by the CI pipeline).
3.  **Create and Push a Git Tag**:
    Create a Git tag that exactly matches the version in `backend/Cargo.toml`. The tag can optionally be prefixed with `v`.
    ```bash
    # Example for version 0.2.0
    git tag v0.2.0 # or git tag 0.2.0
    git push origin v0.2.0
    ```
4.  **Monitor Pipeline**:
    The GitHub Actions pipeline will automatically trigger. The `build_publish_release` job will verify the version, use `make build` to create the package, and then publish it to PyPI.

### Required GitHub Secrets

For the pipeline to publish packages, the following secrets must be configured in your GitHub repository settings (Settings -> Secrets and variables -> Actions -> New repository secret):

-   `PYPI_API_TOKEN`: An API token for PyPI, with permissions to upload packages to your project.
-   `TEST_PYPI_API_TOKEN`: An API token for TestPyPI, with permissions to upload packages.

(Note: `TWINE_USERNAME` is set to `__token__` directly in the workflow when using API tokens).
