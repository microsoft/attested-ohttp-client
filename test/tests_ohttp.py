# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import jwt
import time
import json
import requests
import pyohttp
import asyncio
import urllib3
import pytest
import pytest_asyncio
import requests

def decode_token(token):
    print("decoding: ", token)
    claims = jwt.decode(token, options={"verify_signature": False})
    df = []
    for claim, value in claims.items():
        if claim == "x-ms-isolation-tee" or claim == "x-ms-runtime":
            for k,v in value.items():
                df.append({"claim": claim+"."+k, "value": str(v)})
        else:
            df.append({"claim":claim, "value": str(value)})
    df

@pytest.fixture(scope="module", params=["https://accconfinferenceprod.confidential-ledger.azure.com"])
def ohttp_client(request):
  return pyohttp.OhttpClient(request.param, "/tmp/service_cert.pem")
   
@pytest.mark.asyncio
async def test_basic(ohttp_client, target_uri, api_key, audio_file):
  form_fields = {"file": "@" + audio_file, "response_format": "json" }
  outer_headers = { "api-key": api_key }
  headers = {}
  response = await ohttp_client.post(target_uri, headers, form_fields, outer_headers)
  status = response.status()
  assert(status == 200)
