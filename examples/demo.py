# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.

import streamlit as st
import pandas as pd
import jwt
import subprocess
import os
import time
import json
import requests
import pyohttp as po
import asyncio

import urllib3

urllib3.disable_warnings()

def print_token(col, token):
    print("decoding: ", token)
    claims = jwt.decode(token, options={"verify_signature": False})
    df = []
    for claim, value in claims.items():
        if claim == "x-ms-isolation-tee" or claim == "x-ms-runtime":
            for k,v in value.items():
                df.append({"claim": claim+"."+k, "value": str(v)})
        else:
            df.append({"claim":claim, "value": str(value)})
    with col:
        st.dataframe(pd.DataFrame(df), height=500, use_container_width=True)

async def query(kms_url, target_uri, api_key, audio_file_path):
  form_fields = {"file": "@" + audio_file_path, "response_format": "json" }
  outer_headers = { "api-key": api_key, "x-attestation-token": "true" }
  headers = {}
  client = po.OhttpClient(kms_url, "/tmp/service_cert.pem")
  response = await client.post(target_uri, headers, form_fields, outer_headers)
  return response

async def main():
    st.set_page_config (layout="wide")
    st.title('Azure AI Confidential Whisper Sample')
    kms_url = st.text_input("KMS", value="https://accconfinferenceprod.confidential-ledger.azure.com")
    target_uri = st.text_input("Target URI", value="https://confidential-whisper-aoai.openai.azure.com/openai/deployments/whisper/audio/translations?api-version=2024-06-01")
    api_key = st.text_input("API Key", type="password")
    audio_file = st.file_uploader("Upload audio", type=["mp3","wav"])
    
    col1, col2 = st.columns(2)
    if audio_file is not None:
        audio_file_path = "/tmp/" + audio_file.name
        with open(audio_file_path,"wb") as f:
            f.write((audio_file).getbuffer())
        response = await query(kms_url, target_uri, api_key, audio_file_path)
        status = response.status()
        if status == 200:
            with col1:
                st.header("Transcription result")
                while True: 
                    chunk = await response.chunk()
                    if chunk == None:
                        break
                    col1.write(bytes(chunk).decode("utf-8"))
            with col2:
                st.header("Attestation information")
                print_token(col2, response.headers()["x-attestation-token"])
        else:
            print(status)

if __name__ == "__main__":
    asyncio.run(main())
