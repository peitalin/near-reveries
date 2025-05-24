#!/bin/bash
source .env

cd ./passkey_controller
# Deploy the contract
cargo near deploy build-reproducible-wasm $PASSKEY_CONTROLLER_CONTRACT_ID \
	with-init-call new json-args "{\"trusted_account\": \"$NEAR_SIGNER_ACCOUNT_ID\"}" \
	prepaid-gas '100.0 Tgas' \
	attached-deposit '0 NEAR' \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $NEAR_SIGNER_PUBLIC_KEY \
	--signer-private-key $NEAR_SIGNER_PRIVATE_KEY \
	send

cd ../