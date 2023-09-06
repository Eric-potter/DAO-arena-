use cosmwasm_std::{Addr, Attribute, Binary, CosmosMsg, DepsMut, MessageInfo, Response, StdResult};
use cw20::{Cw20CoinVerified, Cw20ReceiveMsg};
use cw721::Cw721ReceiveMsg;
use cw_balance::{BalanceVerified, Cw721CollectionVerified, MemberShare, MemberShareVerified};
use cw_competition::core::CompetitionCoreActivateMsg;
use cw_ownable::{assert_owner, get_ownership};

use crate::{
    query::is_locked,
    state::{
        is_fully_funded, BALANCE, DUE, IS_FUNDED, IS_LOCKED, PRESET_DISTRIBUTION, TOTAL_BALANCE,
    },
    ContractError,
};

// This function refunds the balance of given addresses
fn inner_withdraw(
    deps: DepsMut,
    addrs: Vec<Addr>,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
    is_processing: bool,
) -> Result<Response, ContractError> {
    // Load the key and total_balance from storage if not processing
    let mut total_balance = match is_processing {
        false => TOTAL_BALANCE.load(deps.storage)?,
        true => BalanceVerified::new(),
    };
    let mut msgs = vec![];
    let mut attrs = vec![];

    for addr in addrs {
        // Load the balance of the current address
        if let Some(balance) = BALANCE.may_load(deps.storage, &addr)? {
            // If the balance is empty, skip this address
            if balance.is_empty() {
                continue;
            }

            // Prepare messages for the balance transmit
            msgs.append(&mut balance.transmit_all(
                deps.as_ref(),
                &addr,
                cw20_msg.clone(),
                cw721_msg.clone(),
            )?);

            // Add address as an attribute to the response
            attrs.push(Attribute {
                key: "addr".to_string(),
                value: addr.to_string(),
            });

            // Update the total_balance by subtracting the refunded balance
            BALANCE.remove(deps.storage, &addr);
            if !is_processing {
                total_balance = total_balance.checked_sub(&balance)?;

                IS_FUNDED.save(deps.storage, &addr, &false)?;
                DUE.update(deps.storage, &addr, |x| -> Result<_, ContractError> {
                    if x.is_none() {
                        return Ok(balance);
                    }
                    Ok(x.unwrap().checked_add(&balance)?)
                })?;
            }
        }
    }

    // Save the updated total_balance to storage
    if !is_processing {
        TOTAL_BALANCE.save(deps.storage, &total_balance)?;
    } else {
        TOTAL_BALANCE.remove(deps.storage);
    }

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "withdraw")
        .add_attributes(attrs)
        .add_messages(msgs))
}

// This function handles refunds for the sender
pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    cw20_msg: Option<Binary>,
    cw721_msg: Option<Binary>,
) -> Result<Response, ContractError> {
    if is_locked(deps.as_ref()) {
        return Err(ContractError::Locked {});
    }

    inner_withdraw(deps, vec![info.sender], cw20_msg, cw721_msg, false)
}

/// Sets the distribution for the sender based on the provided distribution map.
///
/// # Arguments
///
/// * `deps` - A mutable reference to the contract's dependencies.
/// * `info` - The information about the sender and funds.
/// * `distribution` - The distribution map with keys as addresses in string format and values as Uint128.
///
/// # Returns
///
/// * `Result<Response, ContractError>` - A result containing a response or a contract error.
pub fn set_distribution(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Vec<MemberShare>,
) -> Result<Response, ContractError> {
    // Convert String keys to Addr
    let validated_distribution = distribution
        .into_iter()
        .map(|x| x.to_verified(deps.as_ref()))
        .collect::<StdResult<Vec<MemberShareVerified>>>()?;

    // Save distribution in the state
    PRESET_DISTRIBUTION.save(deps.storage, &info.sender, &validated_distribution)?;

    Ok(Response::new()
        .add_attribute("action", "set_distribution")
        .add_attribute("sender", info.sender.to_string()))
}

// This function receives native tokens and updates the balance
pub fn receive_native(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let balance = BalanceVerified {
        native: info.funds,
        cw20: vec![],
        cw721: vec![],
    };

    receive_balance(deps, info.sender, balance)
}

// This function receives CW20 tokens and updates the balance
pub fn receive_cw20(
    deps: DepsMut,
    info: MessageInfo,
    cw20_receive_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender_addr = deps.api.addr_validate(&cw20_receive_msg.sender)?;
    let cw20_balance = vec![Cw20CoinVerified {
        address: info.sender,
        amount: cw20_receive_msg.amount,
    }];

    let balance = BalanceVerified {
        native: info.funds,
        cw20: cw20_balance,
        cw721: vec![],
    };

    receive_balance(deps, sender_addr, balance)
}

// This function receives CW721 tokens and updates the balance
pub fn receive_cw721(
    deps: DepsMut,
    info: MessageInfo,
    cw721_receive_msg: Cw721ReceiveMsg,
) -> Result<Response, ContractError> {
    let sender_addr = deps.api.addr_validate(&cw721_receive_msg.sender)?;
    let cw721_balance = vec![Cw721CollectionVerified {
        addr: info.sender,
        token_ids: vec![cw721_receive_msg.token_id],
    }];

    let balance = BalanceVerified {
        native: info.funds,
        cw20: vec![],
        cw721: cw721_balance,
    };

    receive_balance(deps, sender_addr, balance)
}

// This function updates the balance
fn receive_balance(
    deps: DepsMut,
    addr: Addr,
    balance: BalanceVerified,
) -> Result<Response, ContractError> {
    // Verify the addr
    if !DUE.has(deps.storage, &addr) {
        return Err(ContractError::NoneDue {});
    }

    // Update the balance in storage for the given address
    let balance = BALANCE.update(deps.storage, &addr, |x| -> StdResult<_> {
        Ok(balance.checked_add(&x.unwrap_or(BalanceVerified::default()))?)
    })?;

    let due = DUE.load(deps.storage, &addr)?;
    let new_due = due.checked_sub(&balance)?;
    let mut msgs: Vec<CosmosMsg> = vec![];

    if new_due.is_empty() {
        DUE.remove(deps.storage, &addr);

        IS_FUNDED.save(deps.storage, &addr, &true)?;

        // Lock if funded
        if is_fully_funded(deps.as_ref())? {
            IS_LOCKED.save(deps.storage, &true)?;

            let owner = get_ownership(deps.storage)?.owner;
            if owner.is_some() {
                msgs.push(CompetitionCoreActivateMsg {}.into_cosmos_msg(owner.unwrap())?);
            }
        }
    } else {
        DUE.save(deps.storage, &addr, &new_due)?;
    }

    // Update the total balance in storage
    TOTAL_BALANCE.update(deps.storage, |x| -> StdResult<_> {
        Ok(x.checked_add(&balance)?)
    })?;

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "receive_balance")
        .add_attribute("balance", balance.to_string())
        .add_messages(msgs))
}

// This function handles the competition result message.
pub fn distribute(
    deps: DepsMut,
    info: MessageInfo,
    distribution: Option<Vec<MemberShare>>,
    remainder_addr: String,
) -> Result<Response, ContractError> {
    assert_owner(deps.storage, &info.sender)?;

    if !is_fully_funded(deps.as_ref())? {
        return Err(ContractError::NotFunded {});
    }

    if distribution.is_some() {
        let distribution = distribution.unwrap();

        // Calculate the distributable balance.
        let total_balance = TOTAL_BALANCE.load(deps.storage)?;

        // Validate the remainder address.
        let remainder_addr = deps.api.addr_validate(&remainder_addr)?;

        // Validate the provided distribution.
        let validated_distribution = distribution
            .iter()
            .map(|x| x.to_verified(deps.as_ref()))
            .collect::<StdResult<_>>()?;

        // Calculate the splits based on the distributable total and the validated distribution.
        let distributed_amounts = total_balance.split(&validated_distribution, &remainder_addr)?;

        // Clear the existing balance storage.
        BALANCE.clear(deps.storage);

        // Save the new balances based on the calculated splits.
        for distributed_amount in distributed_amounts {
            // Check if there is a preset distribution for the address
            if let Some(preset) =
                PRESET_DISTRIBUTION.may_load(deps.storage, &distributed_amount.addr)?
            {
                // If there is a preset distribution, apply it to the balance
                let new_balances = distributed_amount
                    .balance
                    .split(&preset, &distributed_amount.addr)?;

                // Save the new balances
                for new_balance in new_balances {
                    BALANCE.save(deps.storage, &new_balance.addr, &new_balance.balance)?;
                }
            } else {
                // If there is no preset distribution, save the balance as is
                BALANCE.save(
                    deps.storage,
                    &distributed_amount.addr,
                    &distributed_amount.balance,
                )?;
            }
        }
    }

    IS_LOCKED.save(deps.storage, &false)?;

    // Free up space
    DUE.clear(deps.storage);
    IS_FUNDED.clear(deps.storage);
    PRESET_DISTRIBUTION.clear(deps.storage);

    // Return the response with the added action attribute.
    let keys = BALANCE
        .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .try_fold(Vec::new(), |mut acc, res| match res {
            Ok(addr) => {
                acc.push(addr);
                Ok(acc)
            }
            Err(e) => Err(e),
        })?;
    let response = inner_withdraw(deps, keys, None, None, true)?;
    Ok(response.add_attribute("action", "handle_competition_result"))
}

// This function handles the competition state change message
pub fn lock(deps: DepsMut, info: MessageInfo, value: bool) -> Result<Response, ContractError> {
    assert_owner(deps.storage, &info.sender)?;

    // Save the locked state to storage
    IS_LOCKED.save(deps.storage, &value)?;

    // Build and return the response
    Ok(Response::new()
        .add_attribute("action", "handle_competition_state_changed")
        .add_attribute("is_locked", value.to_string()))
}