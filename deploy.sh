#!/bin/bash
source .env

# Deploy the contract
cargo near deploy build-reproducible-wasm $CONTRACT_ACCOUNT_ID \
	without-init-call \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $NEAR_SIGNER_PUBLIC_KEY \
	--signer-private-key $NEAR_SIGNER_PRIVATE_KEY \
	send

# Initialize the contract
near call $CONTRACT_ACCOUNT_ID new '{"trusted_account": "$NEAR_SIGNER_ACCOUNT_ID"}' --accountId $NEAR_SIGNER_ACCOUNT_ID --networkId testnet
