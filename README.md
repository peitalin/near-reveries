# near-reveries

Install [`cargo-near`](https://github.com/near/cargo-near) and run:
```bash
cargo near build
```
Tests
```bash
cargo test
```

## Useful Commands

#### Create sub-account
```
near create-account <payments.your-account.testnet> --masterAccount <your-account.testnet> --initialBalance 1
```

#### Initialize the contract
```
near call <contract.testnet> new '{"trusted_account": "your-account.testnet"}' --accountId <your-account.testnet> --networkId testnet
```

#### Delete contract and refund to master account
```
near delete <contract-account.testnet> <beneficiary-account.testnet>
```

#### Re-create the contract
```
near create-account <contract-account.testnet> --masterAccount <your-account.testnet> --initialBalance <amount>
```
or
```
near account create-account fund-myself <contract-account.testnet> '1 NEAR' autogenerate-new-keypair save-to-keychain sign-as <your-account.testnet> network-config testnet sign-with-keychain send
```

#### List keys after re-creating the account
After the main account deploys a contract subaccount, get the contract keys with:
```
near keys <contract-account.testnet>
near account export-account <contract-account.testnet> using-private-key network-config testnet
```
You will need this if deleting and re-creating a contract (e.g. upgrading a testnet contract with different storage layouts)

#### Create a dev account with the newer cargo-near tool
```
cargo near create-dev-account
```


## Deployment
Deployment is automated with GitHub Actions CI/CD pipeline.
To deploy manually, install [`cargo-near`](https://github.com/near/cargo-near) and run:
```bash
cargo near deploy build-reproducible-wasm <account-id>
```

