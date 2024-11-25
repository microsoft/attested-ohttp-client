#!/bin/bash

# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

cd pyohttp

# Create a virtual environment
python3 -m venv ./.env
source .env/bin/activate

# Install paturin
pip3 install maturin[patchelf]

# Build package
maturin build

deactivate