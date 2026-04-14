# Python reference prototype

This package is the readable reference model. It is for:

- deterministic state-transition tests
- protocol experiments
- test vector generation
- VM semantics validation

It is **not** intended for public value-bearing deployment.

## Install

```bash
python3 -m venv .venv
source .venv/bin/activate
python3 -m pip install -U pip
python3 -m pip install -e .[dev]
```

## Run tests

```bash
pytest
```

## Demo

```bash
python3 -m axiom_py.cli demo
```
