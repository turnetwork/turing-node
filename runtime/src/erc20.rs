/// A simple implementation of the ERC20
use parity_codec::{Codec, Decode, Encode};
use rstd::prelude::Vec;
use runtime_primitives::traits::{Hash, As, CheckedAdd, CheckedSub, Member, SimpleArithmetic};
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
        Owners get(owners): map T::Hash => T::AccountId;
        TokenCount get(token_count): u64 = 0;
        Tokens get(token_details): map T::Hash => Token<T::TokenBalance>;
        Balances get(balance_of): map (T::Hash, T::AccountId) => T::TokenBalance;
        Allowances get(allowance): map (T::Hash, T::AccountId, T::AccountId) => T::TokenBalance;

        TokenHash get(token_hash): map u64 => T::Hash;
        TokenIndex : map T::Hash => u64;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event<T>() = default;

        /// Transfers token from the sender to the `to` address.
        fn transfer(origin, token_hash: T::Hash, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            Self::transfer_impl(token_hash, sender, to, value)
        }

        /// Approve the passed address to spend the specified amount of tokens on the behalf of the message's sender.
        fn approve(origin, token_hash: T::Hash, spender: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let owner = ensure_signed(origin)?;

            ensure!(<Balances<T>>::exists((token_hash.clone(), owner.clone())), "Account does not own this token");

            ensure!(spender != owner, "Owner is implicitly approved");

            let allowance = Self::allowance((token_hash.clone(), owner.clone(), spender.clone()));
            let new_allowance = allowance.checked_add(&value).ok_or("overflow in adding allowance")?;

            <Allowances<T>>::insert((token_hash.clone(), owner.clone(), spender.clone()), new_allowance);

            Self::deposit_event(RawEvent::Approval(token_hash, owner, spender, value));
            Ok(())
        }

        /// Transfer tokens from one address to another by allowance
        fn transfer_from(origin, token_hash: T::Hash, from: T::AccountId, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            // Need to be authorized first
            let caller = ensure_signed(origin)?;
            ensure!(<Allowances<T>>::exists((token_hash.clone(), from.clone(), caller.clone())), "Need to be approved first.");
            let allowance = Self::allowance((token_hash.clone() ,from.clone(), caller.clone()));
            ensure!(allowance >= value, "Not enough allowance.");

            let new_allowance = allowance.checked_sub(&value).ok_or("underflow in subtracting allowance.")?;
            <Allowances<T>>::insert((token_hash.clone(), from.clone(), caller.clone()), new_allowance);

            Self::deposit_event(RawEvent::Approval(token_hash.clone(), from.clone(), caller.clone(), new_allowance));
            Self::transfer_impl(token_hash, from, to, value)
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
                name: name.clone(),
                symbol,
                total_supply,
                decimal,
            };
            let id = Self::token_count();
            let hash = (sender.clone(), id, name).using_encoded(<T as system::Trait>::Hashing::hash);

            <Balances<T>>::insert((hash.clone(), sender.clone()), total_supply);
            <Tokens<T>>::insert(hash.clone(), t);
            <Owners<T>>::insert(hash.clone(), sender.clone());

            
            <TokenIndex<T>>::insert(hash.clone(), id);
            <TokenHash<T>>::insert(id, hash.clone());

            <TokenCount<T>>::mutate(|i| *i += 1);
            Self::deposit_event(RawEvent::CreateToken(sender, hash));
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as self::Trait>::TokenBalance,
        Hash = <T as system::Trait>::Hash,
    {
        Transfer(Hash, AccountId, AccountId, Balance),
        Approval(Hash, AccountId, AccountId, Balance),
        CreateToken(AccountId, Hash),
    }
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    pub fn transfer_impl(
        token_hash: T::Hash,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> Result {
        ensure!(
            <Balances<T>>::exists((token_hash.clone(), from.clone())),
            "Account does not own this token"
        );
        let balance_from = Self::balance_of((token_hash.clone(), from.clone()));
        ensure!(balance_from >= value, "Not enough balance.");

        // update the balances
        let new_balance_from = balance_from
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of((token_hash.clone(), to.clone()));
        let new_balance_to = balance_to
            .checked_add(&value)
            .ok_or("overflow in adding balance")?;

        <Balances<T>>::insert((token_hash.clone(), from.clone()), new_balance_from);
        <Balances<T>>::insert((token_hash.clone(), to.clone()), new_balance_to);

        Self::deposit_event(RawEvent::Transfer(token_hash, from, to, value));
        Ok(())
    }
}
