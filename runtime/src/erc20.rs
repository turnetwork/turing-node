/// A simple implementation of the ERC20
use parity_codec::{Codec, Decode, Encode};
use rstd::prelude::Vec;
use runtime_primitives::traits::{As, CheckedAdd, CheckedSub, Member, SimpleArithmetic};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter, StorageMap, StorageValue
};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBalance: Parameter
        + Member
        + SimpleArithmetic
        + Codec
        + Default
        + Copy
        + As<usize>
        + As<u64>;
}

// struct to store the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Token<TokenBalance> {
    name: Vec<u8>,
    symbol: Vec<u8>,
    total_supply: TokenBalance,
    decimal: u64,
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ERC20 {
        Owners get(owners): map u64 => T::AccountId;
        TokenID get(token_id): u64 = 0;
        Tokens get(token_details): map u64 => Token<T::TokenBalance>;
        Balances get(balance_of): map (u64, T::AccountId) => T::TokenBalance;
        Allowances get(allowance): map (u64, T::AccountId, T::AccountId) => T::TokenBalance;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event<T>() = default;

        /// Transfers token from the sender to the `to` address.
        fn transfer(origin, id: u64, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            Self::transfer_impl(id, sender, to, value)
        }

        /// Approve the passed address to spend the specified amount of tokens on the behalf of the message's sender.
        fn approve(origin, id: u64, spender: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let owner = ensure_signed(origin)?;

            ensure!(<Balances<T>>::exists((id, owner.clone())), "Account does not own this token");

            ensure!(spender != owner, "Owner is implicitly approved");

            let allowance = Self::allowance((id, owner.clone(), spender.clone()));
            let new_allowance = allowance.checked_add(&value).ok_or("overflow in adding allowance")?;

            <Allowances<T>>::insert((id, owner.clone(), spender.clone()), new_allowance);

            Self::deposit_event(RawEvent::Approval(owner, spender, value));
            Ok(())
        }

        /// Transfer tokens from one address to another by allowance
        fn transfer_from(origin, id: u64, from: T::AccountId, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            // Need to be authorized first
            let caller = ensure_signed(origin)?;
            ensure!(<Allowances<T>>::exists((id, from.clone(), caller.clone())), "Need to be approved first.");
            let allowance = Self::allowance((id ,from.clone(), caller.clone()));
            ensure!(allowance >= value, "Not enough allowance.");

            let new_allowance = allowance.checked_sub(&value).ok_or("underflow in subtracting allowance.")?;
            <Allowances<T>>::insert((id, from.clone(), caller.clone()), new_allowance);

            Self::deposit_event(RawEvent::Approval(from.clone(), caller.clone(), value));
            Self::transfer_impl(id, from, to, value)
        }

        fn create_token(
            origin,
            name: Vec<u8>,
            symbol: Vec<u8>,
            #[compact] total_supply: T::TokenBalance,
            decimal: u64
            ) -> Result {
            let sender = ensure_signed(origin)?;
            let t = Token {
                name,
                symbol,
                total_supply,
                decimal,
            };
            let id = Self::token_id();

            <Balances<T>>::insert((id, sender.clone()), total_supply);
            <Tokens<T>>::insert(id, t);
            <Owners<T>>::insert(id, sender);

            <TokenID<T>>::mutate(|i| *i += 1);
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as self::Trait>::TokenBalance,
    {
        Transfer(AccountId, AccountId, Balance),
        Approval(AccountId, AccountId, Balance),
    }
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    pub fn transfer_impl(
        id: u64,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> Result {
        ensure!(
            <Balances<T>>::exists((id, from.clone())),
            "Account does not own this token"
        );
        let balance_from = Self::balance_of((id, from.clone()));
        ensure!(balance_from >= value, "Not enough balance.");

        // update the balances
        let new_balance_from = balance_from
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of((id, to.clone()));
        let new_balance_to = balance_to
            .checked_add(&value)
            .ok_or("overflow in adding balance")?;

        <Balances<T>>::insert((id, from.clone()), new_balance_from);
        <Balances<T>>::insert((id, to.clone()), new_balance_to);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }
}
