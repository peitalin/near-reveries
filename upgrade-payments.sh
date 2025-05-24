source .env

cd ./payments
# Deploy contract without initialization call
cargo near deploy build-reproducible-wasm $PAYMENTS_CONTRACT_ID \
	without-init-call \
	network-config testnet \
	sign-with-plaintext-private-key \
	--signer-public-key $NEAR_SIGNER_PUBLIC_KEY \
	--signer-private-key $NEAR_SIGNER_PRIVATE_KEY \
	send

# This only works if storage layout hasn't changed from last deployment
# If storage layout has changed, you must delete and re-create the contract, see README.md
cd ../