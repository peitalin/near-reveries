#!/bin/bash
source .env

cd ./payments
# Deploy the contract
cargo near deploy build-reproducible-wasm $PAYMENTS_CONTRACT_ID \
	with-init-call new json-args "{\"trusted_account\": \"$DEPLOYER_ACCOUNT_ID\"}" \
	prepaid-gas '100.0 Tgas' \
	attached-deposit '0 NEAR' \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $DEPLOYER_PUBLIC_KEY \
	--signer-private-key $DEPLOYER_PRIVATE_KEY \
	send

cd ../