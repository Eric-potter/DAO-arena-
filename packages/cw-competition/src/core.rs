use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Binary, CosmosMsg, StdResult, WasmMsg};

#[cw_serde]
pub struct CompetitionCoreActivateMsg {}

impl CompetitionCoreActivateMsg {
    /// serializes the message
    pub fn into_binary(self) -> StdResult<Binary> {
        let msg = CompetitionCoreExecuteMsg::Activate(self);
        to_binary(&msg)
    }

    /// creates a cosmos_msg sending this struct to the named contract
    pub fn into_cosmos_msg<T: Into<String>>(self, contract_addr: T) -> StdResult<CosmosMsg> {
        let msg = self.into_binary()?;
        let execute = WasmMsg::Execute {
            contract_addr: contract_addr.into(),
            msg,
            funds: vec![],
        };
        Ok(execute.into())
    }
}

#[cw_serde]
enum CompetitionCoreExecuteMsg {
    Activate(CompetitionCoreActivateMsg),
}