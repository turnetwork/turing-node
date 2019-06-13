/// A simple implementation of the DAO.

use crate::lockabletoken as token;
use parity_codec::{Encode, Decode};
use rstd::prelude::*;
use runtime_primitives::traits::{As, CheckedAdd, CheckedSub, Hash};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use {system::ensure_signed, timestamp};

#[cfg(feature = "std")]
use runtime_io::with_storage;

pub trait Trait: timestamp::Trait + token::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
// generic type parameters - Balance, AccountId, timestamp::Moment, Hash
pub struct Proposal<U, V, W, X> {
    // The address where the `amount` will go to if the proposal is accepted
    recipient: V,
    // The amount to transfer to `recipient` if the proposal is accepted.
    amount: U,
    description: Vec<u8>,
    voting_deadline: W,
    open: bool,
    proposal_passed: bool,
    proposal_hash: X,
    // Deposit in wei the creator added when submitting their proposal. It
    // is taken from the msg.value of a new_proposal call.
    proposal_deposit: U,
    new_curator: bool,
    yea: U,
    nay: U,
    creator: V,
}

// storage
decl_storage! {
    trait Store for Module<T: Trait> as Dao {
        // stores the curator in the genesis config
        Curator get(curator) config(): Option<T::AccountId>;

        VoteNo get(vote_no): map T::AccountId => bool;
        VoteYes get(vote_yes): map T::AccountId => bool;

        // DAO parameter begin
        MinProposalDeposit get(min_proposal_deposit) config(): Option<T::TokenBalance>;
        LastTimeMinQuorumMet get(last_time_min_quorum_met): Option<T::Moment>;
        MinQuorumDivisor get(min_quorum_divisor) config(): Option<u32>;
        MinProposalDebatePeriod get(min_proposal_debate_period) config(): Option<T::Moment>;
        QuorumHavlingPeriod get(quorum_havling_period) config(): Option<T::Moment>;
        ExecuteProposalPeriod get(execute_proposal_period) config(): Option<T::Moment>;
        MaxDepositDivisor get(max_deposit_divisor) config(): Option<u32>;
        // DAO parameter end

        Proposals get(proposals): map u64 => Proposal<T::TokenBalance, T::AccountId, T::Moment, T::Hash>;
        ProposalCount get(proposal_count): u64;

        AllowedRecipients get(allowed_recipients): map T::AccountId => bool;
        // Map of addresses blocked during a vote (not allowed to transfer DAO
        // tokens). The address points to the proposal ID.
        Blocked get(blocked): map T::AccountId => u64;
        // Map of addresses and proposal voted on by this address
        VotingRegister get(voting_register): map (T::AccountId, u64) => u64;
        VotingRegisterCount get(voting_register_count): map T::AccountId => u64;
        SumOfProposalDeposits get(sum_of_proposal_deposits): T::TokenBalance;
    }

    // initialize the DAO
    // initialize token
    // make sender an admin if it's the curator account set in genesis config
    // curator then has all the tokens and admin rights to the DAO
    add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageOverlay, _: &mut runtime_primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            with_storage(storage, || {
                <LastTimeMinQuorumMet<T>>::put(<timestamp::Module<T>>::get());
                <ProposalCount<T>>::put(1);
                <AllowedRecipients<T>>::insert(config.curator.clone(), true);
                //<Module<T>>::init().unwrap();
            });
        })
    }
}

// events
decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId,
        Balance = <T as token::Trait>::TokenBalance
    {
        ProposalAdded(u64, AccountId, Balance, Vec<u8>),
        ProposalTaillied(u64, bool, Balance),
        // when a proposal is voted on
        Voted(u64, bool, AccountId),
        AllowedRecipientChanged(AccountId, bool),
    }
);

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    // initialize events for this module
    fn deposit_event<T>() = default;

    fn init(origin) -> Result {
        let curator = Self::curator().ok_or("curator not set?")?;
        let sender = ensure_signed(origin)?;
        ensure!(sender == curator, "Only the curator set in genesis config can initialize");
        <token::Module<T>>::init(curator)?;
        Ok(())
    }

    fn new_proposal(
        origin,
        recipient: T::AccountId,
        amount: T::TokenBalance,
        description: Vec<u8>,
        transaction_data: Vec<u8>,
        debating_period: T::Moment,
        deposit: T::TokenBalance
    ) -> Result{
        let sender = ensure_signed(origin)?;
        ensure!(<token::Module<T>>::balance_of(sender.clone()) > T::TokenBalance::sa(0), "This account doesn't hold the token");

        ensure!(Self::allowed_recipients(recipient.clone()), "The recipient is not in whitelist");
            
        let min_proposal_debate_period  = Self::min_proposal_debate_period().ok_or("MinProposalDebatePeriod not set.")?;
        ensure!(debating_period > min_proposal_debate_period, "debating_period too short");
        ensure!(debating_period < T::Moment::sa(8*7*24*3600), "debating_period too long");

        let min_deposit = Self::min_proposal_deposit().ok_or("MinProposalDeposit not set?")?;
        ensure!(deposit > min_deposit, "deposit should be more than min_deposit");

        let count = Self::proposal_count();
        // to prevent curator from halving quorum before first proposal
        if count ==1 {
            <LastTimeMinQuorumMet<T>>::put(<timestamp::Module<T>>::get());
        }

        // safe?
        let proposal_id = count;
        <ProposalCount<T>>::put(count + 1);

        let voting_deadline = <timestamp::Module<T>>::get().checked_add(&debating_period).ok_or("Overflow when setting voting deadline.")?;
        let proposal_hash = <T as system::Trait>::Hashing::hash(&(transaction_data));

        let p = Proposal{
            recipient: recipient.clone(),
            amount,
            description: description.clone(),
            voting_deadline,
            open: true,
            proposal_passed: false,
            proposal_hash,
            proposal_deposit: deposit,
            new_curator: false,
            yea: T::TokenBalance::sa(0),
            nay: T::TokenBalance::sa(0),
            creator: sender.clone(),
        };

        let sum = Self::sum_of_proposal_deposits();
        let new_sum = sum.checked_add(&deposit).ok_or("Overflow in calculating sumOfProposalDeposits.")?;
        <SumOfProposalDeposits<T>>::put(new_sum);

        <Proposals<T>>::insert(proposal_id, p);
        <token::Module<T>>::lock(sender, deposit, proposal_id)?;

        Self::deposit_event(RawEvent::ProposalAdded(
            proposal_id,
            recipient,
            amount,
            description
        ));

        Ok(())
    }

    fn vote(origin, proposal_id: u64, supports_proposal: bool) -> Result{
        let sender = ensure_signed(origin)?;
        ensure!(Self::blocked(sender.clone()) == 0
               || Self::proposals(proposal_id).voting_deadline > Self::proposals(Self::blocked(sender.clone())).voting_deadline,
               "This is blocked");
        Self::_unvote(sender.clone(), proposal_id)?;

        <Proposals<T>>::mutate(proposal_id, |p| {
            <Blocked<T>>::insert(sender.clone(), proposal_id);
            if supports_proposal {
                p.yea += <token::Module<T>>::balance_of(sender.clone());
                <VoteYes<T>>::insert(sender.clone(), true);
            } else {
                p.nay += <token::Module<T>>::balance_of(sender.clone());
                <VoteNo<T>>::insert(sender.clone(), true);
            }
        });

        let voting_register_count = Self::voting_register_count(sender.clone());
        <VotingRegister<T>>::insert((sender.clone(), voting_register_count), proposal_id);
        <VotingRegisterCount<T>>::insert(sender.clone(), voting_register_count + 1);

        Self::deposit_event(RawEvent::Voted(proposal_id, supports_proposal, sender));

        Ok(())
    }

    fn execute_proposal(proposal_id: u64, transaction_data: Vec<u8>) -> Result{
        let execute_proposal_period = Self::execute_proposal_period().ok_or("execute_proposal_period not set?")?;
        let p = Self::proposals(proposal_id);
        let now = <timestamp::Module<T>>::get();
        
        if p.open && now > p.voting_deadline.clone() + execute_proposal_period {
            Self::close_proposal(proposal_id)?;
            ensure!(false,"The execution deadline has passed.");
        }

        ensure!(now >= p.voting_deadline, "It has not yet reached the voting deadline.");
        ensure!(p.open, "Proposal not open");
        ensure!(!p.proposal_passed, "Proposal has already been passed");
        ensure!(p.proposal_hash == <T as system::Trait>::Hashing::hash(&transaction_data), "Not match the proposal hash");

        if !Self::allowed_recipients(p.recipient.clone()) {
            Self::close_proposal(proposal_id)?;
            <token::Module<T>>::unlock(p.creator.clone(), p.proposal_deposit, proposal_id)?;
            return Err("No such recipient in the whitelist.");
        }

        let mut proposal_check = true;
        let actual_balance = Self::actual_balance();
        if p.amount > actual_balance {
            proposal_check = false;
        }
        let quorum = p.yea;

        // Need improved
        let min_quorum_divisor = Self::min_quorum_divisor().ok_or("min_quorum_divisor not set?")?;
        if transaction_data.len() >= 4 && quorum < Self::min_quorum(min_quorum_divisor, actual_balance)
        {
            proposal_check = false;
        }

        if quorum >= Self::min_quorum(min_quorum_divisor, p.amount) {
            <token::Module<T>>::unlock(p.creator.clone(), p.proposal_deposit, proposal_id)?;
            <LastTimeMinQuorumMet<T>>::put(now);
            if quorum > <token::Module<T>>::total_supply() / T::TokenBalance::sa(7) {
                <MinQuorumDivisor<T>>::put(7);
            }
        }

        <Proposals<T>>::mutate(proposal_id, |p|{
            if quorum >= Self::min_quorum(min_quorum_divisor, p.amount) && p.yea > p.nay && proposal_check {
                p.proposal_passed = true;
            }
        });
        Self::close_proposal(proposal_id)?;

        <token::Module<T>>::lock(Self::curator().unwrap(), p.amount, proposal_id)?;
        <token::Module<T>>::unlock(p.recipient.clone(), p.amount, proposal_id)?;
        
        Self::deposit_event(RawEvent::ProposalTaillied(proposal_id, true, quorum));

        Ok(())
    }

    fn change_min_proposal_deposit(origin, new_min_proposal_deposit: T::TokenBalance) -> Result{
        let sender = ensure_signed(origin)?;
        if sender != Self::curator().unwrap() {
            return Err("Only curator can change min_proposal_deposit");
        }
        let max_deposit_divisor = Self::max_deposit_divisor().ok_or("max_deposit_divisor not set?")?;
        if sender != Self::curator().unwrap() || new_min_proposal_deposit > Self::actual_balance() / T::TokenBalance::sa(max_deposit_divisor.into())
        {
            return Err("change_proposal_deposit failed");
        }
        <MinProposalDeposit<T>>::put(new_min_proposal_deposit);
        Ok(())
    }

    fn change_allowed_recipients(origin, recipient: T::AccountId, allowed: bool) ->Result{
        let sender = ensure_signed(origin)?;
        if sender != Self::curator().unwrap() {
            return Err("Only curator can change whitelist");
        }
        <AllowedRecipients<T>>::insert(recipient.clone(), allowed);
        Self::deposit_event(RawEvent::AllowedRecipientChanged(recipient, allowed));
        Ok(())
    }

    fn halvemin_quorum(origin) -> Result {
        let sender = ensure_signed(origin)?;
        let now = <timestamp::Module<T>>::get();
        let quorum_havling_period = Self::quorum_havling_period().ok_or("quorum_havling_period not set ?")?;
        let min_proposal_debate_period = Self::min_proposal_debate_period().ok_or("min_proposal_debate_period not set?")?;
        let last_time_min_quorum_met = Self::last_time_min_quorum_met().ok_or("last_time_min_quorum_met not set?")?;
        let min_quorum_divisor = Self::min_quorum_divisor().ok_or("minQuorumDivisor not set?")?;

        if (last_time_min_quorum_met < (now.clone() - quorum_havling_period) || sender == Self::curator().unwrap())
            && last_time_min_quorum_met < (now.clone() - min_proposal_debate_period)
            && Self::proposal_count() > 1 {
            <LastTimeMinQuorumMet<T>>::put(now);
            <MinQuorumDivisor<T>>::put(min_quorum_divisor * 2u32);
            return Ok(());
        } else {
            return Err("halvemin_quorum failed.");
        }
    }

    fn unblock_me(origin) -> Result {
        let sender = ensure_signed(origin)?;
        ensure!(Self::get_or_modify_blocked(sender), "can not modify blocked account");
        Ok(())
    }
  }
}

// implementation of mudule
// utility and private functions
impl<T: Trait> Module<T> {
    fn _unvote(sender: T::AccountId, proposal_id: u64) -> Result{
        ensure!(<timestamp::Module<T>>::get() < Self::proposals(proposal_id).voting_deadline, "Already past voting deadling");
        <Proposals<T>>::mutate(proposal_id, |p| {
            if Self::vote_yes(sender.clone()) {
                p.yea -= <token::Module<T>>::balance_of(sender.clone());
                <VoteYes<T>>::insert(sender.clone(), false);
            }

            if Self::vote_no(sender.clone()) {
                p.nay -= <token::Module<T>>::balance_of(sender.clone());
                <VoteNo<T>>::insert(sender.clone(), false);
            }
        });

        Ok(())
    }

    fn close_proposal(proposal_id: u64) -> Result{
        let mut p = Self::proposals(proposal_id).clone();
        if p.open {
            let sum = Self::sum_of_proposal_deposits();
            let new_sum = sum.checked_sub(&p.proposal_deposit).ok_or("Underflow when setting sum_of_proposal_deposits.")?;
            <SumOfProposalDeposits<T>>::put(new_sum);
        }
        p.open = false;
        <Proposals<T>>::insert(proposal_id, p);
        Ok(())
    }

    // actualBalance must not underflow
    fn actual_balance() -> T::TokenBalance {
        let balance = <token::Module<T>>::balance_of(Self::curator().unwrap());
        balance - Self::sum_of_proposal_deposits()
    }

    fn min_quorum(min_quorum_divisor: u32, value: T::TokenBalance) -> T::TokenBalance {
        <token::Module<T>>::total_supply() / T::TokenBalance::sa(min_quorum_divisor.into()) + 
        (value * <token::Module<T>>::total_supply()) / (T::TokenBalance::sa(3) * Self::actual_balance())
    }

    fn get_or_modify_blocked(account: T::AccountId) -> bool{
        if Self::blocked(account.clone()) == 0 {
            return false;
        }
        let p = Self::proposals(Self::blocked(account.clone()));
        if !p.open {
            <Blocked<T>>::insert(account, 0);
            return false;
        } else {
            return true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use primitives::{Blake2Hasher, H256};
    use runtime_io::with_externalities;
    use runtime_primitives::{
        testing::{Digest, DigestItem, Header, UintAuthorityId},
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };
    use support::{assert_noop, assert_ok, impl_outer_origin};

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    impl system::Trait for Test {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type Digest = Digest;
        type AccountId = u64;
        type Lookup = IdentityLookup<u64>;
        type Header = Header;
        type Event = ();
        type Log = DigestItem;
    }
    impl consensus::Trait for Test {
        type Log = DigestItem;
        type SessionKey = UintAuthorityId;
        type InherentOfflineReport = ();
    }
    impl token::Trait for Test {
        type Event = ();
        type TokenBalance = u64;
    }
    impl timestamp::Trait for Test {
        type Moment = u64;
        type OnTimestampSet = ();
    }
    impl Trait for Test {
        type Event = ();
    }
    type Dao = Module<Test>;
    type Token = token::Module<Test>;
    type Timestamp = timestamp::Module<Test>;

    fn init() -> Result {
        Dao::init(Origin::signed(1))
    }

    // builds the genesis config store and sets mock values
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            token::GenesisConfig::<Test> { 
                total_supply: 21000000,
                name: "ABMatrix Token".as_bytes().into(),
			    symbol: "ABT".as_bytes().into(),
			    decimal: 18, 
            }
                .build_storage()
                .unwrap()
                .0,
        ); 
        t.extend(
            GenesisConfig::<Test> {
                curator: 1,
			    min_proposal_deposit: 100,
			    min_quorum_divisor: 7,
			    min_proposal_debate_period: 14,
			    quorum_havling_period: 175,
			    execute_proposal_period: 10,
			    max_deposit_divisor: 100,
            }
                .build_storage()
                .unwrap()
                .0,
        );
        t.into()
    }

    #[test]
    fn should_init(){
       with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_eq!(Dao::curator().unwrap(), 1);
            assert_eq!(Dao::last_time_min_quorum_met().unwrap(), 0);
            assert_eq!(Dao::proposal_count(), 1);
            assert_eq!(Dao::allowed_recipients(1), true);
            assert_eq!(Dao::allowed_recipients(2), false);
            assert_eq!(Token::total_supply(), 21000000);
            assert_eq!(Token::balance_of(1), 21000000);
            assert_eq!(Token::balance_of(2), 0);

            assert_eq!(Dao::min_proposal_deposit().unwrap(), 100);
            assert_eq!(Dao::min_quorum_divisor().unwrap(), 7);
            assert_eq!(Dao::min_proposal_debate_period().unwrap(), 14);
            assert_eq!(Dao::quorum_havling_period().unwrap(), 175);
            assert_eq!(Dao::execute_proposal_period().unwrap(), 10);
            assert_eq!(Dao::max_deposit_divisor().unwrap(), 100);
        });
    }

    #[test]
    fn should_fail_insufficient_balance(){
        with_externalities(&mut new_test_ext(), || {
        assert_ok!(init());
            assert_noop!(
            Dao::new_proposal(
                Origin::signed(2),
                1,
                10,
                "description".as_bytes().into(),
                "transaction_data".as_bytes().into(),
                15,
                101
            ),
            "This account doesn't hold the token"
            );
        });
    }

    #[test]
    fn should_fail_not_allowed_recipients(){
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_noop!(
            Dao::new_proposal(
                Origin::signed(1),
                2,
                10,
                "description".as_bytes().into(),
                "transaction_data".as_bytes().into(),
                15,
                101
            ),
            "The recipient is not in whitelist"
            );
        });
    }

    #[test]
    fn should_fail_short_debating_period(){
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_noop!(
            Dao::new_proposal(
                Origin::signed(1),
                1,
                10,
                "description".as_bytes().into(),
                "transaction_data".as_bytes().into(),
                13,
                101
            ),
            "debating_period too short"
            );
        });
    }

    #[test]
    fn should_fail_long_debating_period(){
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_noop!(
            Dao::new_proposal(
                Origin::signed(1),
                1,
                10,
                "description".as_bytes().into(),
                "transaction_data".as_bytes().into(),
                8*7*24*3600+1,
                101
            ),
            "debating_period too long"
            );
        });
    }

    #[test]
    fn should_fail_low_deposit() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_noop!(
            Dao::new_proposal(
                Origin::signed(1),
                1,
                10,
                "description".as_bytes().into(),
                "transaction_data".as_bytes().into(),
                15,
                100
            ),
            "deposit should be more than min_deposit"
            );
        });
    }

    #[test]
    fn should_pass_proposal() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_ok!(
                Dao::new_proposal(
                    Origin::signed(1),
                    1,
                    10,
                    "description".as_bytes().into(),
                    "transaction_data".as_bytes().into(),
                    15,
                    101
                )
            );
        });
    }

    #[test]
    fn should_pass_vote() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_ok!(
                Dao::new_proposal(
                    Origin::signed(1),
                    1,
                    10,
                    "description".as_bytes().into(),
                    "transaction_data".as_bytes().into(),
                    15,
                    101
                )
            );
            assert_ok!(Dao::vote(Origin::signed(1), 1, true));
            assert_eq!(Dao::proposals(1).yea, 21000000-101);
            assert_eq!(Dao::vote_yes(1), true);
            assert_eq!(Dao::blocked(1), 1);
            assert_eq!(Dao::voting_register((1, 0)), 1);
            assert_eq!(Dao::voting_register_count(1), 1);
        });
    }

    #[test]
    fn should_pass_change_allowed_recipients(){
       with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_ok!(Dao::change_allowed_recipients(Origin::signed(1), 2, true));
            assert_eq!(Dao::allowed_recipients(2), true);
       }); 
    }

    #[test]
    fn should_pass_execute_proposal() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_ok!(Dao::change_allowed_recipients(Origin::signed(1), 2, true));
            assert_ok!(
                Dao::new_proposal(
                    Origin::signed(1),
                    2,
                    10,
                    "description".as_bytes().into(),
                    "transaction_data".as_bytes().into(),
                    15,
                    101
                )
            );
            assert_ok!(Dao::vote(Origin::signed(1), 1, true));
            Timestamp::set_timestamp(16);
            assert_ok!(Dao::execute_proposal(1, "transaction_data".as_bytes().into()));
            assert_eq!(Dao::proposals(1).proposal_passed, true);
            assert_eq!(Token::balance_of(2), 10);
            assert_eq!(Token::balance_of(1), 21000000-10);
            assert_eq!(Dao::proposals(1).open, false);
        });
    }

    #[test]
    fn should_pass_unblock_me() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(init());
            assert_ok!(
                Dao::new_proposal(
                    Origin::signed(1),
                    1,
                    10,
                    "description".as_bytes().into(),
                    "transaction_data".as_bytes().into(),
                    15,
                    101
                )
            );
            assert_ok!(Dao::vote(Origin::signed(1), 1, true));
            assert_ok!(Dao::unblock_me(Origin::signed(1)));
        });
    }

}