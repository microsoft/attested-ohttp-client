# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import asyncio
import json
import pyohttp
import pytest
import pytest_asyncio
import requests

def download_kms_certificate(kms_url, output_file):
  response = requests.get(kms_url + "/node/network", verify=False)
  if response.status_code == 200:
    service_certificate = json.loads(response.text).get("service_certificate", "")
    if service_certificate:
        with open(output_file, "w") as file:
            file.write(service_certificate)
    else:
        assert False
  else:
    assert False


@pytest.fixture(scope="module", params=["https://accconfinferenceproduction.confidential-ledger.azure.com"])
def ohttp_client(request):
  output_file = "/tmp/service_cert.pem"
  download_kms_certificate(kms_url=request.param, output_file=output_file)
  return pyohttp.OhttpClient(request.param, output_file)


@pytest.mark.asyncio
async def test_basic(ohttp_client, target_uri, api_key, audio_file):
  form_fields = {"file": "@" + audio_file, "response_format": "json" }
  outer_headers = { "api-key": api_key }
  response = await ohttp_client.post(target_uri, form_fields=form_fields, outer_headers=outer_headers)
  status = response.status()
  for key, value in response.headers().items():
    print(f"{key}: {value}")
  assert status == 200


@pytest.mark.asyncio
async def test_attestation_token(ohttp_client, target_uri, api_key, audio_file):
  form_fields = {"file": "@" + audio_file, "response_format": "json" }
  outer_headers = { "api-key": api_key, "x-attestation-token": "true" }
  response = await ohttp_client.post(target_uri, form_fields=form_fields, outer_headers=outer_headers)
  status = response.status()
  for key, value in response.headers().items():
    print(f"{key}: {value}")
  assert "x-attestation-token" in response.headers()
  assert status == 200


@pytest.mark.asyncio
async def test_invalid_api_key(ohttp_client, target_uri, audio_file):
  form_fields = {"file": "@" + audio_file, "response_format": "json" }
  outer_headers = { "api-key": "invalid_key" }
  response = await ohttp_client.post(target_uri, form_fields=form_fields, outer_headers=outer_headers)
  status = response.status()
  for key, value in response.headers().items():
    print(f"{key}: {value}")
  assert status == 401
