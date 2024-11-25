# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

def pytest_addoption(parser):
    parser.addoption(
        "--kms_url",
        action="append",
        default=[],
        help="KMS URL",
    )
    parser.addoption(
        "--target_uri",
        action="append",
        default=[],
        help="target uri",
    )
    parser.addoption(
        "--api_key",
        action="append",
        default=[],
        help="API Key",
    )
    parser.addoption(
        "--audio_file",
        action="append",
        default=[],
        help="Path to audio file",
    )

def pytest_generate_tests(metafunc):
    if "kms_url" in metafunc.fixturenames:
        metafunc.parametrize("kms_url", metafunc.config.getoption("kms_url"))
    if "target_uri" in metafunc.fixturenames:
        metafunc.parametrize("target_uri", metafunc.config.getoption("target_uri"))
    if "api_key" in metafunc.fixturenames:
        metafunc.parametrize("api_key", metafunc.config.getoption("api_key"))
    if "audio_file" in metafunc.fixturenames:
        metafunc.parametrize("audio_file", metafunc.config.getoption("audio_file"))