#!/bin/bash
source .env

cargo near deploy build-reproducible-wasm $NEAR_SIGNER_ACCOUNT_ID \
	without-init-call \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $NEAR_SIGNER_PUBLIC_KEY \
	--signer-private-key $NEAR_SIGNER_PRIVATE_KEY \
	send
