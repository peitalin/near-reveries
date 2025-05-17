#!/bin/bash
source .env

# Deploy the contract
cargo near deploy build-reproducible-wasm $CONTRACT_ACCOUNT_ID \
	with-init-call new '{"trusted_account": "$NEAR_SIGNER_ACCOUNT_ID"}' \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $NEAR_SIGNER_PUBLIC_KEY \
	--signer-private-key $NEAR_SIGNER_PRIVATE_KEY \
	send

