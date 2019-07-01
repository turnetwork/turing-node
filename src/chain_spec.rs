use primitives::{ed25519, sr25519, Pair, crypto::UncheckedInto};
use turing_node_runtime::{AccountId, ConsensusConfig, SessionConfig, StakingConfig, StakerStatus, TimestampConfig, BalancesConfig,
	SudoConfig, ContractConfig, GrandpaConfig, IndicesConfig, Permill, Perbill, GenesisConfig, TreasuryConfig, DemocracyConfig,
	CouncilSeatsConfig, CouncilVotingConfig, ERC20Config, ERC721Config, DaoTokenConfig, DaoConfig};
use substrate_service;
use hex_literal::{hex, hex_impl};
use ed25519::Public as AuthorityId;
use telemetry::TelemetryEndpoints;
use std::marker::PhantomData;

// Note this is the URL for the telemetry server
const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
	/// Whatever the current runtime is, with just Alice as an auth.
	Development,
	/// Whatever the current runtime is, with simple Alice/Bob auths.
	LocalTestnet,

	/// Custom config
	TuringTestnet,
}

fn authority_key(s: &str) -> AuthorityId {
	ed25519::Pair::from_string(&format!("//{}", s), None)
		.expect("static values are valid; qed")
		.public()
}

fn account_key(s: &str) -> AccountId {
	sr25519::Pair::from_string(&format!("//{}", s), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, AuthorityId) {
	(
		account_key(&format!("{}//stash", seed)),
		account_key(seed),
		authority_key(seed)
	)
}

impl Alternative {
	/// Get an actual chain config from one of the alternatives.
	pub(crate) fn load(self) -> Result<ChainSpec, String> {
		Ok(match self {
			Alternative::Development => ChainSpec::from_genesis(
				"Development",
				"dev",
				|| testnet_genesis(vec![
					get_authority_keys_from_seed("Alice")
				], vec![
					account_key("Alice"),
					account_key("Alice//stash"),
				],
					account_key("Alice")
				),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::LocalTestnet => ChainSpec::from_genesis(
				"Local Testnet",
				"local_testnet",
				|| testnet_genesis(vec![
					get_authority_keys_from_seed("Alice"),
					get_authority_keys_from_seed("Bob"),
				], vec![
					account_key("Alice"),
					account_key("Bob"),
					account_key("Charlie"),
					account_key("Dave"),
					account_key("Eve"),
					account_key("Ferdie"),
					account_key("Alice//stash"),
					account_key("Bob//stash"),
					account_key("Charlie//stash"),
					account_key("Dave//stash"),
					account_key("Eve//stash"),
					account_key("Ferdie//stash"),
				],
					account_key("Alice"),
				),
				vec![],
				None,
				None,
				None,
				None
			),
			Alternative::TuringTestnet => ChainSpec::from_genesis(
				"Turing Testnet v0.1.0", 
				"turing", 
				turing_testnet_config_genesis,
				vec![],
				Some(TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])),
				None,
				None,
				None
			),
		})
	}

	pub(crate) fn from(s: &str) -> Option<Self> {
		match s {
			"dev" => Some(Alternative::Development),
			"" | "local" => Some(Alternative::LocalTestnet),
			"turing" => Some(Alternative::TuringTestnet),
			_ => None,
		}
	}
}

const MILLICENTS: u128 = 1_000_000_000;
const CENTS: u128 = 1_000 * MILLICENTS;    // assume this is worth about a cent.
const DOLLARS: u128 = 100 * CENTS;

const SECS_PER_BLOCK: u64 = 10;
const MINUTES: u64 = 60 / SECS_PER_BLOCK;
const HOURS: u64 = MINUTES * 60;
const DAYS: u64 = HOURS * 24;
const WEEKS: u64 = DAYS * 7;

const STASH: u128 = 100 * DOLLARS;

fn testnet_genesis(initial_authorities: Vec<(AccountId, AccountId, AuthorityId)>, endowed_accounts: Vec<AccountId>, root_key: AccountId) -> GenesisConfig {
	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/turing_node_runtime_wasm.compact.wasm").to_vec(),
			authorities: initial_authorities.iter().map(|x| x.2.clone()).collect(),
		}),
		system: None,
		balances: Some(BalancesConfig {
			transaction_base_fee: 1 * CENTS,
			transaction_byte_fee: 10 * MILLICENTS,
			balances: endowed_accounts.iter().cloned().map(|k|(k.clone(), 1 << 60)).collect(),
			existential_deposit: 1 * DOLLARS,
			transfer_fee: 1 * CENTS,
			creation_fee: 1 * CENTS,
			vesting: vec![],
		}),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.clone(),
		}),
		session: Some(SessionConfig {
			validators: initial_authorities.iter().map(|x| x.1.clone()).collect(),
			session_length: 5 * MINUTES,
			keys: initial_authorities.iter().map(|x| (x.1.clone(), x.2.clone())).collect::<Vec<_>>(),
		}),
		staking: Some(StakingConfig {
			current_era: 0,
			offline_slash: Perbill::from_billionths(1_000_000),
			session_reward: Perbill::from_billionths(2_065),
			current_session_reward: 0,
			validator_count: 5,
			sessions_per_era: 12,
			bonding_duration: 60 * MINUTES,
			offline_slash_grace: 4,
			minimum_validator_count: 2,
			stakers: initial_authorities.iter().map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.1.clone()).collect(),
		}),
		democracy: Some(DemocracyConfig {
			launch_period: 10 * MINUTES,    // 1 day per public referendum
			voting_period: 10 * MINUTES,    // 3 days to discuss & vote on an active referendum
			minimum_deposit: 50 * DOLLARS,    // 12000 as the minimum deposit for a referendum
			public_delay: 10 * MINUTES,
			max_lock_periods: 6,
		}),
		council_seats: Some(CouncilSeatsConfig {
			active_council: vec![],
			candidacy_bond: 10 * DOLLARS,
			voter_bond: 1 * DOLLARS,
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
			minimum_period: SECS_PER_BLOCK / 2, // due to the nature of aura the slots are 2*period
		}),
		treasury: Some(TreasuryConfig {
			proposal_bond: Permill::from_percent(5),
			proposal_bond_minimum: 1 * DOLLARS,
			spend_period: 1 * DAYS,
			burn: Permill::from_percent(50),
		}),
		contract: Some(ContractConfig {
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
			key: root_key,
		}),
		grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
		}),
		erc20: Some(ERC20Config {
			// set Alice as owner
			owner: account_key("Alice"),
			total_supply: 21000000,
			name: "ABMatrix ERC20 Token".as_bytes().into(),
			symbol: "ABT20".as_bytes().into(),
		}),
		erc721: Some(ERC721Config {
			_genesis_phantom_data: PhantomData,
			name: "ABMatrix ERC721 Token".as_bytes().into(),
			symbol: "ABT721".as_bytes().into(),
		}),
		daotoken: Some(DaoTokenConfig{
			total_supply: 21000000,
			name: "ABMatrix Token".as_bytes().into(),
			symbol: "ABT".as_bytes().into(),
			decimal: 18,
		}),
		dao: Some(DaoConfig {
			// set Alice as curator
			curator: account_key("Alice"),
			min_proposal_deposit: 100,
			min_quorum_divisor: 7,
			min_proposal_debate_period: 2 * WEEKS,
			quorum_havling_period: 25 * WEEKS,
			execute_proposal_period: 10 * DAYS,
			max_deposit_divisor: 100,
		}),
	}
}

fn turing_testnet_config_genesis() -> GenesisConfig {
	// stash, controller, session-key
	// generated with secret:
	// for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/elm/$j/$i; done; done
	// and
	// for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//elm//$j//$i; done; done


	let initial_authorities: Vec<(AccountId, AccountId, AuthorityId)> = vec![(
		hex!["889bb56aeb50bedf6cb59943c6a7bde3e7436922a5b67d0dddafa1120674e459"].unchecked_into(), // 5F9pdBTUoEV3UCqxDZhGzkLsvWUwTTFd83Y8wPubw9jAAbQX
		hex!["56d9c6e3c808e58778c06d120da724c73311e2b3aa66715bb4200710e03f444a"].unchecked_into(), // 5E2afj6FrPMEnL4iCiByWy2qEH77GBh6mzn6xHwRmZpC5Y2a
		hex!["7df9261c4d0981469b311f204fc930087298c3413979a64465ca364371a77ee8"].unchecked_into(), // 5EusrZtoVy2KJWsqTiwuizjUXX1kmgJFeZM6FdguPkdLPBEL
	),(
		hex!["a21f0be38ba6b85f3d37b45976ae94cb537b4f854e5c23a070dba5efc2450224"].unchecked_into(), // 5FjGqWbnhdZVfNJeCff3P1gpvuLJCTztUrjfTxxL6546F282
		hex!["b6d913612c513f1f1e29d3d2964f8e6c890f47b7aa83b1e5d52b78a234a88537"].unchecked_into(), // 5GCT4GSJ38HvnJPE6Pzc1Nef9yEvEdx9Po6oZXDNtdfNLsva
		hex!["7573ddee17e983f08fd84576e78b23da3f7ea50863638abfb400e6694a7fb1dc"].unchecked_into(), // 5EihsxLFA7DAiZ6nx381hvrGjuESHNVjGRcvkxhWbw4ECkVm
	),(
		hex!["f4dcf18a7091236034337af748a1bde3b4b725cc0a869ce51cb5526270615606"].unchecked_into(), // 5HbmB6q13VeCevJzG65Gh4Uim8X6mCjEdUusWzu76jCK6Y3t
		hex!["4e556650785a8fa5f4d20d108d6110fd470cb6e2d2e3c092ab831c28e9a6ee1f"].unchecked_into(), // 5DqQxhcsKoR66TEA3dZQ2Gw3xFgLnnj4prndzxXy7aTDcVHN
		hex!["8dcb3bae9227881fb5cf00a219a4e2bc4b8fee4c18d7b90dbdb19a711c928314"].unchecked_into(), // 5FGcytsmKbhWeqrrDV8RmfLShC5EuaJzVjvW39LyyBLRtyqy
	),(
		hex!["14509c5ee333dc714caf3f55b58b407b0d42c2fdd61c9c40a19ed28d88f49c1b"].unchecked_into(), // 5CXLm7AiMrpjWzJfmQ79JMA2LFnrEFkUcbGNAyB2WZ5U3KgY
		hex!["64b9a4fca0c64f1f27933ed45be0aba5c9495b77c5f1225c556ba7897a3df42c"].unchecked_into(), // 5ELmnynqTeNvV9TmBU1fq62HbsDZNYGx4SWn2uPAWc6ERP6i
		hex!["33c9ca7d41b5445dbb344296f538db040d692011f6e65805b136f06ea5be22fa"].unchecked_into(), // 5DEcF68U4eyDjwJgm2h7HTanXUxbbVW8QibJt2Tgzby6grGY
	)];

	// root account
	let endowed_accounts: Vec<AccountId> = vec![
		hex!["94bb1b1234dd3713e568412b6012ad9418792f4c838a6ebc431c2d72b75a8462"].unchecked_into(), //5FRiXU6fNtDyePmukE6PPmBdjwtzcccHFd2CpsN9dZ8tTDQF
	];

	const MILLICENTS: u128 = 1_000_000_000;
	const CENTS: u128 = 1_000 * MILLICENTS;    // assume this is worth about a cent.
	const DOLLARS: u128 = 100 * CENTS;

	const SECS_PER_BLOCK: u64 = 6;
	const MINUTES: u64 = 60 / SECS_PER_BLOCK;
	const HOURS: u64 = MINUTES * 60;
	const DAYS: u64 = HOURS * 24;

	const ENDOWMENT: u128 = 10_000_000 * DOLLARS;
	const STASH: u128 = 100 * DOLLARS;

	GenesisConfig {
		consensus: Some(ConsensusConfig {
			code: include_bytes!("../runtime/wasm/target/wasm32-unknown-unknown/release/turing_node_runtime_wasm.compact.wasm").to_vec(),    // FIXME change once we have #1252
			authorities: initial_authorities.iter().map(|x| x.2.clone()).collect(),
		}),
		system: None,
		balances: Some(BalancesConfig {
			transaction_base_fee: 1 * CENTS,
			transaction_byte_fee: 10 * MILLICENTS,
			balances: endowed_accounts.iter().cloned()
				.map(|k| (k, ENDOWMENT))
				.chain(initial_authorities.iter().map(|x| (x.0.clone(), STASH)))
				.collect(),
			existential_deposit: 1 * DOLLARS,
			transfer_fee: 1 * CENTS,
			creation_fee: 1 * CENTS,
			vesting: vec![],
		}),
		indices: Some(IndicesConfig {
			ids: endowed_accounts.iter().cloned()
				.chain(initial_authorities.iter().map(|x| x.0.clone()))
				.collect::<Vec<_>>(),
		}),
		session: Some(SessionConfig {
			validators: initial_authorities.iter().map(|x| x.1.clone()).collect(),
			session_length: 5 * MINUTES,
			keys: initial_authorities.iter().map(|x| (x.1.clone(), x.2.clone())).collect::<Vec<_>>(),
		}),
		staking: Some(StakingConfig {
			current_era: 0,
			offline_slash: Perbill::from_billionths(1_000_000),
			session_reward: Perbill::from_billionths(2_065),
			current_session_reward: 0,
			validator_count: 7,
			sessions_per_era: 12,
			bonding_duration: 60 * MINUTES,
			offline_slash_grace: 4,
			minimum_validator_count: 4,
			stakers: initial_authorities.iter().map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator)).collect(),
			invulnerables: initial_authorities.iter().map(|x| x.1.clone()).collect(),
		}),
		democracy: Some(DemocracyConfig {
			launch_period: 10 * MINUTES,    // 1 day per public referendum
			voting_period: 10 * MINUTES,    // 3 days to discuss & vote on an active referendum
			minimum_deposit: 50 * DOLLARS,    // 12000 as the minimum deposit for a referendum
			public_delay: 10 * MINUTES,
			max_lock_periods: 6,
		}),
		council_seats: Some(CouncilSeatsConfig {
			active_council: vec![],
			candidacy_bond: 10 * DOLLARS,
			voter_bond: 1 * DOLLARS,
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
			minimum_period: SECS_PER_BLOCK / 2, // due to the nature of aura the slots are 2*period
		}),
		treasury: Some(TreasuryConfig {
			proposal_bond: Permill::from_percent(5),
			proposal_bond_minimum: 1 * DOLLARS,
			spend_period: 1 * DAYS,
			burn: Permill::from_percent(50),
		}),
		contract: Some(ContractConfig {
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
			key: endowed_accounts[0].clone(),
		}),
		grandpa: Some(GrandpaConfig {
			authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
		}),
		erc20: Some(ERC20Config {
			// set Alice as owner
			owner: endowed_accounts[0].clone(),
			total_supply: 21000000,
			name: "ABMatrix ERC20 Token".as_bytes().into(),
			symbol: "ABT20".as_bytes().into(),
		}),
		erc721: Some(ERC721Config {
			_genesis_phantom_data: PhantomData,
			name: "ABMatrix ERC721 Token".as_bytes().into(),
			symbol: "ABT721".as_bytes().into(),
		}),
		daotoken: Some(DaoTokenConfig{
			total_supply: 21000000,
			name: "ABMatrix Token".as_bytes().into(),
			symbol: "ABT".as_bytes().into(),
			decimal: 18,
		}),
		dao: Some(DaoConfig {
			// set Alice as curator
			curator: endowed_accounts[0].clone(),
			min_proposal_deposit: 100,
			min_quorum_divisor: 7,
			min_proposal_debate_period: 2 * WEEKS,
			quorum_havling_period: 25 * WEEKS,
			execute_proposal_period: 10 * DAYS,
			max_deposit_divisor: 100,
		}),
	}
}
