use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Deps, StdResult, Uint128, Uint64};
use cw_balance::MemberShare;
use cw_storage_plus::{Item, Map};
use cw_utils::Expiration;

#[cw_serde]
pub struct Match {
    pub match_number: Uint128,
    pub team_1: Addr,
    pub team_2: Addr,
    pub result: Option<Result>,
}

#[cw_serde]
pub enum Result {
    Team1,
    Team2,
    Draw,
}

#[cw_serde]
pub struct Round {
    pub round_number: Uint64,
    pub matches: Vec<Uint128>, // A link to the Match by match_number
    pub expiration: Expiration,
}

impl Round {
    pub fn to_response(self, deps: Deps, league_id: Uint128) -> StdResult<RoundResponse> {
        let matches = MATCHES
            .prefix((league_id.u128(), self.round_number.u64()))
            .range(deps.storage, None, None, cosmwasm_std::Order::Descending)
            .map(|x| x.map(|y| y.1))
            .collect::<StdResult<Vec<Match>>>()?;

        Ok(RoundResponse {
            round_number: self.round_number,
            matches,
            expiration: self.expiration,
        })
    }
}

#[cw_serde]
pub struct RoundResponse {
    pub round_number: Uint64,
    pub matches: Vec<Match>,
    pub expiration: Expiration,
}

/// (League Id, Round Number)
pub const ROUNDS: Map<(u128, u64), Round> = Map::new("rounds");
/// (League Id, Round Number, Match Number)
pub const MATCHES: Map<(u128, u64, u128), Match> = Map::new("matches");
pub const DISTRIBUTION: Item<Vec<MemberShare<Addr>>> = Item::new("distribution");
