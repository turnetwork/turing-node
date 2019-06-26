use crate::lockabletoken as token;
use parity_codec::{Encode, Decode};
use rstd::prelude::*;
use runtime_primitives::traits::{As, Bounded};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageValue, StorageMap
};
use support::traits::{WithdrawReasons, LockableCurrency};
use {system::ensure_signed, timestamp};

pub trait Trait: timestamp::Trait + token::Trait{
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Currency: LockableCurrency<Self::AccountId, Moment=Self::BlockNumber>;
}

type Balance<T> = <<T as Trait>::Currency as support::traits::Currency<<T as system::Trait>::AccountId>>::Balance;

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
pub struct Crowdsale<AccountId, TokenBalance, Moment> {
    beneficiary: AccountId,
    funding_goal: TokenBalance,
    amount_raised: TokenBalance,
    deadline: Moment,
    funding_goal_reached: bool,
    crowdsale_closed: bool,
    price: u32,
}

const PAY_ID: [u8; 8] = *b"exchange";

decl_storage! {
	trait Store for Module<T: Trait> as DaoToken {
        Crowdsales get(crowdsales) : map u32 => Crowdsale<T::AccountId, T::TokenBalance, T::Moment>;
        CrowdsaleCount get(crowdsale_count) : u32 = 0;
    }
}

// events
decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId,
        TokenBalance = <T as token::Trait>::TokenBalance,
        Balance = <<T as Trait>::Currency as support::traits::Currency<<T as system::Trait>::AccountId>>::Balance,
    {
        CreateCrowdsale(u32, AccountId),
        // crowdsale_id, recipient, totalAmountRaised
        GoalReached(u32, AccountId, TokenBalance),
        // crowdsale_id, backer, amount, isContribution
        FundUnlock(u32, AccountId, Option<TokenBalance>),
        FundLock(u32, AccountId, TokenBalance),

        PayToken(u32, AccountId, Balance),
    }
);

decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
    // initialize events for this module
    fn deposit_event<T>() = default;
    
    // TODO: set deposit invoid being attacked
    pub fn create_crowdsale(
        origin,
        if_successful_send_to: T::AccountId,
        funding_goal_in_turs: T::TokenBalance,
        duration_in_seconds: T::Moment,
        price: u32,
        token_name: Vec<u8>,
        token_symbol: Vec<u8>,
        token_total_supply: T::TokenBalance,
        token_decimal: u32) -> Result {
            let sender = ensure_signed(origin)?;
            let c = Crowdsale{
                beneficiary: if_successful_send_to,
                funding_goal: funding_goal_in_turs,
                amount_raised: T::TokenBalance::sa(0),
                deadline: duration_in_seconds + <timestamp::Module<T>>::get(),
                funding_goal_reached: false,
                crowdsale_closed: false,
                price
            };

            let id = Self::crowdsale_count();
            <Crowdsales<T>>::insert(id, c);
            <CrowdsaleCount<T>>::mutate(|i| *i += 1);

            let create_token_result = <token::Module<T>>::create_token(sender.clone(), id, token_name, token_symbol, token_total_supply, token_decimal);
            if create_token_result.is_ok() {
                Self::deposit_event(RawEvent::CreateCrowdsale(id, sender));
            } 

            create_token_result
    }

    // TODO: exchange balance to tokens
    pub fn pay(origin, crowdsale_id: u32, value: T::TokenBalance) -> Result{
        let sender = ensure_signed(origin)?;

        let c = Self::crowdsales(crowdsale_id);
        ensure!(!c.crowdsale_closed, "crowsale has already been closed");

        let value_to_tokenbalance = <Balance<T> as As<u64>>::sa(T::TokenBalance::as_(value));
 
        // lock balance
        T::Currency::set_lock(
            PAY_ID,
            &sender,
            value_to_tokenbalance,
            T::BlockNumber::max_value(),
            WithdrawReasons::all()
        );

        // add to token
        let owner = <token::Module<T>>::owners(crowdsale_id);

        let tranfer_impl_result = <token::Module<T>>::transfer_impl(crowdsale_id, owner, sender.clone(), value / T::TokenBalance::sa(c.price.into()));
        
        if tranfer_impl_result.is_ok(){
            Self::deposit_event(RawEvent::PayToken(crowdsale_id, sender, value_to_tokenbalance));
        }

        Ok(())
    }

    pub fn invest(origin, crowdsale_id: u32, amount: T::TokenBalance) -> Result {
        let sender = ensure_signed(origin)?;

        let mut c = Self::crowdsales(crowdsale_id);
        ensure!(!c.crowdsale_closed, "crowsale has already been closed");

        let lock_result = <token::Module<T>>::lock(crowdsale_id, sender.clone(), amount);

        if lock_result.is_ok(){
            c.amount_raised += amount;
            <Crowdsales<T>>::insert(crowdsale_id, c);

            Self::deposit_event(RawEvent::FundLock(crowdsale_id, sender, amount));
        }

        lock_result
    }

    pub fn check_goal_reached(crowdsale_id: u32) -> Result {
        let mut c = Self::crowdsales(crowdsale_id);
        ensure!(<timestamp::Module<T>>::get() >= c.deadline, "It's not the deadline yet");

        if c.amount_raised >= c.funding_goal {
            c.funding_goal_reached = true;
            Self::deposit_event(RawEvent::GoalReached(crowdsale_id, c.clone().beneficiary, c.clone().amount_raised));
        }

        c.crowdsale_closed = true;
        <Crowdsales<T>>::insert(crowdsale_id, c);

        Ok(())
    }

    pub fn distribute(origin, crowdsale_id: u32) -> Result {
        let c = Self::crowdsales(crowdsale_id);
        let sender = ensure_signed(origin)?;

        if !c.funding_goal_reached {
            if <token::Module<T>>::unlock(crowdsale_id, sender.clone(), None).is_ok() {
                Self::deposit_event(RawEvent::FundUnlock(crowdsale_id, sender.clone(), None));
            }
        }

        if c.funding_goal_reached && c.beneficiary == sender.clone() {
            if <token::Module<T>>::unlock(crowdsale_id, c.beneficiary, Some(c.amount_raised)).is_ok() {
                Self::deposit_event(RawEvent::FundUnlock(crowdsale_id, sender, Some(c.amount_raised)));
            }
        }

        Ok(())
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
    impl balances::Trait for Test {
	    type Balance = u64;
        type OnFreeBalanceZero = ();
	    type OnNewAccount = ();
	    type Event = ();
	    type TransactionPayment = ();
	    type TransferPayment = ();
	    type DustRemoval = ();
    }
    impl Trait for Test {
        type Event = ();
        type Currency = balances::Module<Self>;
    }
    
    type Ico = Module<Test>;
    type Token = token::Module<Test>;
    type Timestamp = timestamp::Module<Test>;
    type Balances = balances::Module<Test>;

    // builds the genesis config store and sets mock values
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let mut t = system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .0;
        t.extend(
            balances::GenesisConfig::<Test> {
                balances: vec![(1, 200)],
                transaction_base_fee: 0,
			    transaction_byte_fee: 0,
			    existential_deposit: 1,
			    transfer_fee: 0,
			    creation_fee: 0,
			    vesting: vec![],
            }
                .build_storage()
                .unwrap()
                .0,
        );
        
        t.into()
    }

    #[test]
    fn should_create_success() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Ico::create_crowdsale(
                Origin::signed(1),
                2,
                100,
                10,
                1,
                "ABMatrix Token".as_bytes().into(),
                "ABT".as_bytes().into(),
                1000,
                18
                )
            );
        });
    }

    #[test]
    fn check_pay() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Ico::create_crowdsale(
                Origin::signed(1),
                2,
                100,
                10,
                1,
                "ABMatrix Token".as_bytes().into(),
                "ABT".as_bytes().into(),
                1000,
                18
                )
            );
            assert_ok!(Ico::pay(Origin::signed(1), 0, 100));

            // check lock
            assert_noop!(Balances::transfer(Origin::signed(1), 2, 200), "account liquidity restrictions prevent withdrawal");

            assert_eq!(Token::balance_of((0, 1)), 1100);
        });
    }

    #[test]
    fn check_invest() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Ico::create_crowdsale(
                Origin::signed(1),
                2,
                100,
                10,
                1,
                "ABMatrix Token".as_bytes().into(),
                "ABT".as_bytes().into(),
                1000,
                18
                )
            );
            assert_ok!(Ico::pay(Origin::signed(1), 0, 100));
            assert_ok!(Ico::invest(Origin::signed(1), 0, 100));
        });
    }

    #[test]
    fn should_pass_check_goal_reached(){
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Ico::create_crowdsale(
                Origin::signed(1),
                2,
                100,
                10,
                1,
                "ABMatrix Token".as_bytes().into(),
                "ABT".as_bytes().into(),
                1000,
                18
            ));
            assert_ok!(Ico::pay(Origin::signed(1), 0, 100));
            assert_ok!(Ico::invest(Origin::signed(1), 0, 100));

            Timestamp::set_timestamp(11);
            assert_ok!(Ico::check_goal_reached(0));
        });
    }

    #[test]
    fn should_pass_distribute_goal_reached() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Ico::create_crowdsale(
                Origin::signed(1),
                2,
                100,
                10,
                1,
                "ABMatrix Token".as_bytes().into(),
                "ABT".as_bytes().into(),
                1000,
                18
            ));
            assert_ok!(Ico::pay(Origin::signed(1), 0, 100));
            assert_ok!(Ico::invest(Origin::signed(1), 0, 100));

            Timestamp::set_timestamp(11);
            assert_ok!(Ico::check_goal_reached(0));
            let c = Ico::crowdsales(0);
            assert_eq!(c.funding_goal_reached, true);

            assert_ok!(Ico::distribute(Origin::signed(2), 0));
            assert_eq!(Token::balance_of((0, 2)), 100);
        });
    }

    #[test]
    fn should_pass_distribute_withdraw() {
        with_externalities(&mut new_test_ext(), || {
            assert_ok!(Ico::create_crowdsale(
                Origin::signed(1),
                2,
                100,
                10,
                1,
                "ABMatrix Token".as_bytes().into(),
                "ABT".as_bytes().into(),
                1000,
                18
            ));
            assert_eq!(Token::balance_of((0, 1)), 1000);
            assert_ok!(Ico::pay(Origin::signed(1), 0, 100));
            assert_eq!(Token::balance_of((0, 1)), 1100);
            assert_ok!(Ico::invest(Origin::signed(1), 0, 99));
            assert_eq!(Token::balance_of((0, 1)), 1001);
            
            assert_eq!(Token::locked_tokens((0, 1)), 99);

            Timestamp::set_timestamp(11);
            assert_ok!(Ico::check_goal_reached(0));

            assert_ok!(Ico::distribute(Origin::signed(1), 0));
            assert_eq!(Token::balance_of((0, 1)), 1100);
        });
    }
}