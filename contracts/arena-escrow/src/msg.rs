use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Binary;
use cw20::Cw20ReceiveMsg;
use cw721::Cw721ReceiveMsg;
#[allow(unused_imports)]
use cw_balance::{BalanceVerified, MemberBalance, MemberShare, MemberShareVerified};
use cw_competition::escrow::CompetitionEscrowDistributeMsg;
use cw_ownable::{cw_ownable_execute, cw_ownable_query};

#[cw_serde]
pub struct InstantiateMsg {
    pub dues: Vec<MemberBalance>,
}

#[cw_ownable_execute]
#[cw_serde]
pub enum ExecuteMsg {
    Withdraw {
        cw20_msg: Option<Binary>,
        cw721_msg: Option<Binary>,
    },
    SetDistribution {
        distribution: Vec<MemberShare>,
    },
    ReceiveNative {},
    Receive(Cw20ReceiveMsg),
    ReceiveNft(Cw721ReceiveMsg),
    Distribute(CompetitionEscrowDistributeMsg),
    Lock {
        value: bool,
    },
}

#[cw_ownable_query]
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<(String, BalanceVerified)>)]
    Balances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(BalanceVerified)]
    Balance { addr: String },
    #[returns(BalanceVerified)]
    Due { addr: String },
    #[returns(Vec<(String, BalanceVerified)>)]
    Dues {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(bool)]
    IsFunded { addr: String },
    #[returns(bool)]
    IsFullyFunded {},
    #[returns(BalanceVerified)]
    TotalBalance {},
    #[returns(bool)]
    IsLocked {},
    #[returns(Option<Vec<MemberShareVerified>>)]
    Distribution { addr: String },
}

#[cw_serde]
pub enum MigrateMsg {
    FromV1 {},
}