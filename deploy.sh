#!/bin/bash
source .env

cargo near deploy build-reproducible-wasm yellow-loong.testnet \
	without-init-call \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $NEAR_SIGNER_PUBLIC_KEY \
	--signer-private-key $NEAR_SIGNER_PRIVATE_KEY \
	send
