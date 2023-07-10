Python wrapper for Netwayste v2
===============================

This allows interactively testing Netwayste v2 in a Jupyter notebook.

## Installation

Create a new virtual environment in this folder, activate it in the current terminal, and finally install needed Python modules.

```
python -m venv venv
source venv/bin/activate
pip install ipykernel maturin
```

## Usage

Activate the virtual environment created above in the current terminal, and compile the Rust parts of the `nwv2_python_wrapper` Python module.

```
source venv/bin/activate
maturin develop
```

Then you can start up `jupyter-notebook` and actually import and use the Python module. To reload, re-run `maturin develop`, and if successful, do `Kernel->Restart and Run All` on the Jupyter Notebook page.
