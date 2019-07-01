use primitives::{ed25519, sr25519, Pair};
use turing_node_runtime::{AccountId, ConsensusConfig, SessionConfig, StakingConfig, StakerStatus, TimestampConfig, BalancesConfig,
	SudoConfig, ContractConfig, GrandpaConfig, IndicesConfig, Permill, Perbill, GenesisConfig, TreasuryConfig, DemocracyConfig,
	CouncilSeatsConfig, CouncilVotingConfig, ERC20Config, ERC721Config, DaoTokenConfig, DaoConfig};
use substrate_service;

use ed25519::Public as AuthorityId;
use std::marker::PhantomData;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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