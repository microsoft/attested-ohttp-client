# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import pyohttp
import requests
import asyncio
import json
import argparse

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

async def infer(target_uri, api_key, audio_file, kms_url):
  ohttp_client = pyohttp.OhttpClient(kms_url, kms_cert_path)
  form_fields = {"file": "@" + audio_file, "response_format": "json" }
  outer_headers = { "api-key": api_key }
  response = await ohttp_client.post(target_uri, form_fields=form_fields, outer_headers=outer_headers)
  print(response.status())
  if response.status() == 200:
    while True:
      result = await response.chunk()
      if result is None:
        break
      print(bytearray(result).decode('utf-8'))
  return

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Process some strings.")
    parser.add_argument("--target-uri", type=str, help="The target URI")
    parser.add_argument("--api-key", type=str, help="The API key")
    parser.add_argument("--audio-file", type=str, help="The audio file path")
    parser.add_argument("--kms-url", type=str, help="The KMS URL")

    args = parser.parse_args()
    kms_cert_path = "/tmp/service_cert.pem"
    download_kms_certificate(args.kms_url, kms_cert_path)
    asyncio.run(infer(args.target_uri, args.api_key, args.audio_file, args.kms_url))
