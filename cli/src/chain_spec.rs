// Copyright 2018-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Substrate chain configurations.

use hex_literal::hex;
use node_primitives::AccountId;
pub use node_runtime::GenesisConfig;
use node_runtime::{
    BalancesConfig, ConsensusConfig, ContractConfig, CouncilSeatsConfig,
    CouncilVotingConfig, DemocracyConfig, GrandpaConfig, IndicesConfig, Perbill, Permill,
    SessionConfig, StakerStatus, StakingConfig, SudoConfig, TimestampConfig, TreasuryConfig
};
use primitives::{
    crypto::{UncheckedInto, UncheckedFrom},
    ed25519,
    ed25519::Public as AuthorityId,
    sr25519, Pair,
};
use substrate_service;

use substrate_telemetry::TelemetryEndpoints;

/// Specialized `ChainSpec`.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

const MICROS: u128 = 1;
const MILLICENTS: u128 = 1_000 * MICROS;
const CENTS: u128 = 1_000 * MILLICENTS; // assume this is worth about a cent.
const TNKS: u128 = 1_000 * CENTS;

const MINUTES: u64 = 60;
const HOURS: u64 = MINUTES * 60;
const DAYS: u64 = HOURS * 24;

struct GenesisConfigBuilder {
	pub sec_per_block: u64,
	pub sec_per_session: u64,
	pub sec_per_era: u64,
	pub stash: u128,
	pub endowment: u128,
	pub root_key: AccountId,
	pub ethereum_public_keys: Vec<Vec<u8>>,
	pub endowed_accounts: Vec<AccountId>,
	pub initial_authorities: Vec<(AccountId, AccountId, AuthorityId)>,
	pub validator_count: u32,
	pub minimum_validator_count: u32,
	pub reward_per_year: u128,
	pub validate_minimum_stake: u128,
	pub nominate_minimum_stake: u128,
	pub bonding_duration: u64,
	pub print: bool,
}

impl Default for GenesisConfigBuilder {
	fn default() -> Self {
//		const SECS_PER_BLOCK: u64 = 8;
//		const SESSION_TIME: u64 = MINUTES * 6; // about 360s
//		const SESSION_LENGTH: u64 =  SESSION_TIME / SECS_PER_BLOCK;
//		const ERA_TIME: u64 = HOURS;
//		const ERA_PER_SESSIONS: u64 = ERA_TIME / SESSION_TIME;

		const ENDOWMENT: u128 = 20_000_000 * TNKS;
		const STASH: u128 = 50_000 * TNKS;
		const REWARDYEAR: u128 = 10_000_000 * TNKS;  // 1000w

		Self {
			sec_per_block: 8,
			sec_per_session: MINUTES * 6,
			sec_per_era: HOURS,
			stash: STASH,
			endowment: ENDOWMENT,
			root_key: AccountId::default(),
			ethereum_public_keys: vec![],
			endowed_accounts: vec![],
			initial_authorities: vec![],
			validator_count: 100,
			minimum_validator_count: 1,
			validate_minimum_stake: 50_000 * TNKS,
			nominate_minimum_stake: 10 * TNKS,
			reward_per_year: REWARDYEAR,
			bonding_duration: 240,
			print: false,
		}
	}
}

impl GenesisConfigBuilder {
	pub fn build(&self) -> GenesisConfig {
		let mut config = GenesisConfig {
			consensus: Some(ConsensusConfig {
				code: include_bytes!("../../runtime/wasm/target/wasm32-unknown-unknown/release/node_runtime.compact.wasm").to_vec(),    // FIXME change once we have #1252
				authorities: self.initial_authorities.iter().map(|x| x.2.clone()).collect(),
			}),
			system: None,
			balances: Some(BalancesConfig {
				transaction_base_fee: 100 * MILLICENTS,
				transaction_byte_fee: 10 * MILLICENTS,
				balances: self.endowed_accounts.iter().cloned()
					.map(|k| (k, self.endowment))
					.chain(self.initial_authorities.iter().map(|x| (x.0.clone(), self.stash)))
					.chain(self.initial_authorities.iter().map(|x| (AccountId::unchecked_from(x.2.clone().0), self.stash))) // FIX oracle no need fee
					.collect(),
				existential_deposit: 1 * CENTS,
				transfer_fee: 1 * CENTS,
				creation_fee: 1 * CENTS,
				vesting: vec![],
			}),
			indices: Some(IndicesConfig {
				ids: self.endowed_accounts.iter().cloned()
					.chain(self.initial_authorities.iter().map(|x| x.0.clone()))
					.chain(self.initial_authorities.iter().map(|x| x.1.clone()))
					.collect::<Vec<_>>(),
			}),
			session: Some(SessionConfig {
				validators: self.initial_authorities.iter().map(|x| x.1.clone()).collect(),
				session_length: self.sec_per_session / self.sec_per_block,
				keys: self.initial_authorities.iter().map(|x| (x.1.clone(), x.2.clone())).collect::<Vec<_>>(),
			}),
			staking: Some(StakingConfig {
				current_era: 0,
				offline_slash: Perbill::from_billionths(1),
				session_reward: Perbill::from_billionths(2_065),
				current_session_reward: 0,
				validator_count: self.validator_count,
				sessions_per_era: self.sec_per_era / self.sec_per_session,
				bonding_duration: self.bonding_duration,
				offline_slash_grace: 4,
				minimum_validator_count: self.minimum_validator_count,
				stakers: self.initial_authorities.iter().map(|x| (x.0.clone(), x.1.clone(), self.stash, StakerStatus::Validator)).collect(),
				invulnerables: self.initial_authorities.iter().map(|x| x.0.clone()).collect(),
				nodeinformation: get_nodeinformation(),
				reward_per_year: self.reward_per_year,
				validate_minimum_stake: self.validate_minimum_stake,
				nominate_minimum_stake: self.nominate_minimum_stake,
			}),
			democracy: Some(DemocracyConfig {
				launch_period: 10 * MINUTES,    // 1 day per public referendum
				voting_period: 10 * MINUTES,    // 3 days to discuss & vote on an active referendum
				minimum_deposit: 50 * TNKS,    // 12000 as the minimum deposit for a referendum
				public_delay: 10 * MINUTES,
				max_lock_periods: 6,
			}),
			council_seats: Some(CouncilSeatsConfig {
				active_council: vec![],
				candidacy_bond: 10 * TNKS,
				voter_bond: 1 * TNKS,
				present_slash_per_voter: 1 * CENTS,
				carry_count: 6,
				presentation_duration: 1 * DAYS,
				approval_voting_period: 2 * DAYS,
				term_duration: 28 * DAYS,
				desired_seats: 0,
				inactive_grace_period: 1,    // one additional vote should go by before an inactive voter can be reaped.
			}),
			council_voting: Some(CouncilVotingConfig {
				cooloff_period: 4 * DAYS,
				voting_period: 1 * DAYS,
				enact_delay_period: 0,
			}),
			timestamp: Some(TimestampConfig {
				minimum_period: self.sec_per_block / 2, // due to the nature of aura the slots are 2*period
			}),
			treasury: Some(TreasuryConfig {
				proposal_bond: Permill::from_percent(5),
				proposal_bond_minimum: 1 * TNKS,
				spend_period: 1 * DAYS,
				burn: Permill::from_percent(50),
			}),
			contract: Some(ContractConfig {
				signed_claim_handicap: 2,
				rent_byte_price: 4,
				rent_deposit_offset: 1000,
				storage_size_offset: 8,
				surcharge_reward: 150,
				tombstone_deposit: 16,
				transaction_base_fee: 1 * CENTS,
				transaction_byte_fee: 10 * MILLICENTS,
				transfer_fee: 1 * CENTS,
				creation_fee: 1 * CENTS,
				contract_fee: 1 * CENTS,
				call_base_fee: 1000,
				create_base_fee: 1000,
				gas_price: 1 * MILLICENTS,
				max_depth: 1024,
				block_gas_limit: 10_000_000,
				current_schedule: Default::default(),
			}),
			sudo: Some(SudoConfig {
				key: self.root_key.clone(),
			}),
			grandpa: Some(GrandpaConfig {
				authorities: self.initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
			}),
		};
		if self.print {
			match config.contract.as_mut() {
				Some(contract_config) => contract_config.current_schedule.enable_println = self.print,
				None => {},
			}
		}
		config
	}
}
/// Helper function to generate AccountId from seed
pub fn get_account_id_from_seed(seed: &str) -> AccountId {
    sr25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate AuthorityId from seed
pub fn get_session_key_from_seed(seed: &str) -> AuthorityId {
    ed25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

fn str_to_vecu8(string:&str) -> Vec<u8>{
	let vecu8 : Vec<u8> = string.into();
	vecu8
}

/// Helper function to get Vec<u8> from String
pub fn get_nodeinfo(seed: &str) -> Vec<u8> {
	str_to_vecu8(seed)
}

/// Helper function to get node information
pub fn get_nodeinformation() -> Vec<(Vec<u8>,Vec<u8>,Vec<u8>)> {
	vec![
		(b"Ali".to_vec(), b"TNKdernetwork.io".to_vec(),b"TNKder".to_vec()),
		(b"Bob".to_vec(), b"TNKdernetwork.io".to_vec(),b"TNKder".to_vec()),
		(b"Dav".to_vec(), b"TNKdernetwork.io".to_vec(),b"TNKder".to_vec()),
		(b"Eva".to_vec(), b"TNKdernetwork.io".to_vec(),b"TNKder".to_vec()),
		(b"Tra".to_vec(), b"TNKdernetwork.io".to_vec(),b"TNKder".to_vec()),
		(b"Glo".to_vec(), b"TNKdernetwork.io".to_vec(),b"TNKder".to_vec()),
		([].to_vec(), [].to_vec(),[].to_vec()),
	]
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuthorityId) {
    (
        get_account_id_from_seed(&format!("{}//stash", seed)),
        get_account_id_from_seed(seed),
        get_session_key_from_seed(seed),
    )
}

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, AuthorityId)>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    enable_println: bool,
) -> GenesisConfig {
    let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed("Alice"),
            get_account_id_from_seed("Bob"),
            get_account_id_from_seed("Charlie"),
            get_account_id_from_seed("Dave"),
            get_account_id_from_seed("Eve"),
            get_account_id_from_seed("Ferdie"),
            get_account_id_from_seed("Alice//stash"),
            get_account_id_from_seed("Bob//stash"),
            get_account_id_from_seed("Charlie//stash"),
            get_account_id_from_seed("Dave//stash"),
            get_account_id_from_seed("Eve//stash"),
            get_account_id_from_seed("Ferdie//stash"),
            hex!("889bb56aeb50bedf6cb59943c6a7bde3e7436922a5b67d0dddafa1120674e459")
                .unchecked_into(),
        ]
    });

	let mut builder = GenesisConfigBuilder::default();
	builder.initial_authorities = initial_authorities;
	builder.root_key = root_key;
	builder.endowed_accounts = endowed_accounts;
	builder.print = enable_println;
	builder.sec_per_block = 4;
	builder.sec_per_session = MINUTES;
	builder.sec_per_era = MINUTES * 3;
	builder.build()
}


fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![get_authority_keys_from_seed("Alice")],
        get_account_id_from_seed("Alice"),
        None,
        true,
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        None,
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            get_authority_keys_from_seed("Alice"),
            get_authority_keys_from_seed("Bob"),
        ],
        get_account_id_from_seed("Alice"),
        None,
        false,
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        None,
    )
}

fn turing_testnet_genesis() -> GenesisConfig {
    // stash , control, session
	let initial_authorities: Vec<(AccountId, AccountId, AuthorityId)> = vec![
		(
			// Alice
			hex!["889bb56aeb50bedf6cb59943c6a7bde3e7436922a5b67d0dddafa1120674e459"]
				.unchecked_into(), // 5F9pdBTUoEV3UCqxDZhGzkLsvWUwTTFd83Y8wPubw9jAAbQX
			hex!["56d9c6e3c808e58778c06d120da724c73311e2b3aa66715bb4200710e03f444a"]
				.unchecked_into(), // 5E2afj6FrPMEnL4iCiByWy2qEH77GBh6mzn6xHwRmZpC5Y2a
			hex!["7df9261c4d0981469b311f204fc930087298c3413979a64465ca364371a77ee8"]
				.unchecked_into(), // 5EusrZtoVy2KJWsqTiwuizjUXX1kmgJFeZM6FdguPkdLPBEL
		),
		(
			// Bob
			hex!["a21f0be38ba6b85f3d37b45976ae94cb537b4f854e5c23a070dba5efc2450224"]
				.unchecked_into(), // 5FjGqWbnhdZVfNJeCff3P1gpvuLJCTztUrjfTxxL6546F282
			hex!["b6d913612c513f1f1e29d3d2964f8e6c890f47b7aa83b1e5d52b78a234a88537"]
				.unchecked_into(), // 5GCT4GSJ38HvnJPE6Pzc1Nef9yEvEdx9Po6oZXDNtdfNLsva
			hex!["7573ddee17e983f08fd84576e78b23da3f7ea50863638abfb400e6694a7fb1dc"]
				.unchecked_into(), // 5EihsxLFA7DAiZ6nx381hvrGjuESHNVjGRcvkxhWbw4ECkVm
		),
		(
			// Charlie
			hex!["f4dcf18a7091236034337af748a1bde3b4b725cc0a869ce51cb5526270615606"]
				.unchecked_into(), // 5HbmB6q13VeCevJzG65Gh4Uim8X6mCjEdUusWzu76jCK6Y3t
			hex!["4e556650785a8fa5f4d20d108d6110fd470cb6e2d2e3c092ab831c28e9a6ee1f"]
				.unchecked_into(), // 5DqQxhcsKoR66TEA3dZQ2Gw3xFgLnnj4prndzxXy7aTDcVHN
			hex!["8dcb3bae9227881fb5cf00a219a4e2bc4b8fee4c18d7b90dbdb19a711c928314"]
				.unchecked_into(), // 5FGcytsmKbhWeqrrDV8RmfLShC5EuaJzVjvW39LyyBLRtyqy
		),
		(
			// Dave
			hex!["14509c5ee333dc714caf3f55b58b407b0d42c2fdd61c9c40a19ed28d88f49c1b"]
				.unchecked_into(), // 5CXLm7AiMrpjWzJfmQ79JMA2LFnrEFkUcbGNAyB2WZ5U3KgY
			hex!["64b9a4fca0c64f1f27933ed45be0aba5c9495b77c5f1225c556ba7897a3df42c"]
				.unchecked_into(), // 5ELmnynqTeNvV9TmBU1fq62HbsDZNYGx4SWn2uPAWc6ERP6i
			hex!["33c9ca7d41b5445dbb344296f538db040d692011f6e65805b136f06ea5be22fa"]
				.unchecked_into(), // 5DEcF68U4eyDjwJgm2h7HTanXUxbbVW8QibJt2Tgzby6grGY
		),
	];

	// root account
	let endowed_accounts: Vec<AccountId> = vec![
		hex!["94bb1b1234dd3713e568412b6012ad9418792f4c838a6ebc431c2d72b75a8462"].unchecked_into(), //5FRiXU6fNtDyePmukE6PPmBdjwtzcccHFd2CpsN9dZ8tTDQF
	];

	let root_key = hex!["94bb1b1234dd3713e568412b6012ad9418792f4c838a6ebc431c2d72b75a8462"].unchecked_into(); //5E4CCLsJ3P1UBXgRdzFEQivMMJEqfg3VBj1tpvx8dsJa2FxQ

	let mut builder = GenesisConfigBuilder::default();
	builder.initial_authorities = initial_authorities;
	builder.root_key = root_key;
	builder.endowed_accounts = endowed_accounts;
	builder.validator_count = 30;
	builder.minimum_validator_count = 4;
	builder.build()
}

// Note this is the URL for the telemetry server
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// turing testnet config
pub fn turing_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Turing Testnet v0.2.0",
        "Turing Testnet",
        turing_testnet_genesis,
        vec![],
		Some(TelemetryEndpoints::new(vec![(
			STAGING_TELEMETRY_URL.to_string(),
			0,
		)])),
        None,
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::Factory;
    use service_test;

    fn local_testnet_genesis_instant() -> GenesisConfig {
        let mut genesis = local_testnet_genesis();
        genesis.timestamp = Some(TimestampConfig { minimum_period: 1 });
        genesis
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            local_testnet_genesis_instant,
            vec![],
            None,
            None,
            None,
            None,
        )
    }

    #[test]
	#[ignore]
    fn test_connectivity() {
        service_test::connectivity::<Factory>(integration_test_config());
    }
}
