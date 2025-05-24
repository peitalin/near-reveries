use serde_json::json;
use near_workspaces::types::NearToken;
use near_sdk::json_types::U128;

#[tokio::test]
async fn test_contract_is_operational() -> Result<(), Box<dyn std::error::Error>> {
    let contract_wasm = near_workspaces::compile_project("./").await?;

    test_full_deposit_and_withdrawal_flow(&contract_wasm).await?;
    Ok(())
}

async fn test_full_deposit_and_withdrawal_flow(contract_wasm: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let sandbox = near_workspaces::sandbox().await?;
    let contract = sandbox.dev_deploy(contract_wasm).await?;

    let user_account = sandbox.dev_create_account().await?;

    // Initialize the contract
    let init_outcome = contract
        .call("new")
        .args_json(json!({"trusted_account": user_account.id()}))
        .transact()
        .await?;
    assert!(init_outcome.is_success(), "Initialization failed: {:#?}", init_outcome.into_result().unwrap_err());

    // 1. Should be able to create a reverie (simple signature based access condition)
    let outcome1 = user_account
        .call(contract.id(), "create_reverie")
        .args_json(json!({
            "reverie_id": "rev1",
            "reverie_type": "type1",
            "description": "desc1",
            "access_condition": json!({
                "type": "Ed25519",
                "value": "pubkey1"
            })
        }))
        .transact()
        .await?;
    assert!(outcome1.is_success(), "{:#?}", outcome1.into_result().unwrap_err());

    // 2. Should be able to deposit to a reverie
    let create_reverie_outcome1 = user_account
        .call(contract.id(), "deposit")
        .deposit(NearToken::from_yoctonear(100))
        .args_json(json!({
            "reverie_id": "rev1"
        }))
        .transact()
        .await?;
    assert!(create_reverie_outcome1.is_success(), "{:#?}", create_reverie_outcome1.into_result().unwrap_err());

    // 3. Should be able to get the balance of a user for a reverie
    let user_balance = contract
        .view("get_balance")
        .args_json(json!({
            "reverie_id": "rev1",
            "user_id": user_account.id()
        })).await?;
    assert_eq!(user_balance.json::<U128>()?, U128(100));

    // 4. Should be able to withdraw from a reverie
    let withdraw_outcome = user_account
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "reverie_id": "rev1",
            "amount": "100" // string for bigint
        }))
        .transact()
        .await?;
    assert!(withdraw_outcome.is_success(), "{:#?}", withdraw_outcome.into_result().unwrap_err());

    // Prepare the access_function_args as a JSON string for the Contract AccessCondition
    let access_args_json_string = json!({
        "reverie_id": "rev1",
        "user_id": user_account.id()
    }).to_string();

    // 5. Should be able to create a reverie with a contract access condition
    let create_reverie_outcome2 = user_account
        .call(contract.id(), "create_reverie")
        .args_json(json!({
            "reverie_id": "rev2",
            "reverie_type": "type2",
            "description": "desc2",
            "access_condition": json!({
                "type": "Contract",
                "value": {
                    "address": contract.id().to_string(), // convert to string
                    "access_function_name": "can_spend",
                    "access_function_args": access_args_json_string
                }
            })
        }))
        .transact()
        .await?;
    assert!(create_reverie_outcome2.is_success(), "{:#?}", create_reverie_outcome2.into_result().unwrap_err());

    Ok(())
}


