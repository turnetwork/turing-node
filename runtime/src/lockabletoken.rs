use rstd::prelude::Vec;
use support::{ensure, Parameter, StorageMap, decl_module, decl_storage, decl_event, dispatch::Result};
use system::ensure_signed;
use parity_codec::{Codec, Encode, Decode};
use runtime_primitives::traits::{As, SimpleArithmetic, Member, CheckedAdd, CheckedSub};

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64>;
}

// struct to store the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Token<TokenBalance> {
    name: Vec<u8>,
    symbol: Vec<u8>,
    total_supply: TokenBalance,
    decimal: u32,
}
 
// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as LockableToken {
        Owners get(owners): map u32 => T::AccountId;
        Tokens get(token_details): map u32 => Token<T::TokenBalance>;
		Balances get(balance_of): map (u32, T::AccountId) => T::TokenBalance;
		Allowances get(allowance): map (u32, T::AccountId, T::AccountId) => T::TokenBalance;

		// special interface
		LockedTokens get(locked_tokens): map (u32, T::AccountId) => T::TokenBalance;
        TotalLocked get(total_locked): map u32 => T::TokenBalance;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// Transfers token from the sender to the `to` address.
		fn transfer(origin, ico_id: u32, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            Self::transfer_impl(ico_id, sender, to, value)
        }

		/// Approve the passed address to spend the specified amount of tokens on the behalf of the message's sender.
        fn approve(origin, ico_id: u32, spender: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let owner = ensure_signed(origin)?;
			ensure!(<Balances<T>>::exists((ico_id, owner.clone())), "Account does not own this token");

			ensure!(spender != owner, "Owner is implicitly approved");

            let allowance = Self::allowance((ico_id, owner.clone(), spender.clone()));
			let new_allowance = allowance.checked_add(&value).ok_or("overflow in adding allowance")?;

			<Allowances<T>>::insert((ico_id, owner.clone(), spender.clone()), new_allowance);

			Self::deposit_event(RawEvent::Approval(owner, spender, value));
            Ok(())
        }

        /// Transfer tokens from one address to another by allowance
        fn transfer_from(origin, ico_id: u32, from: T::AccountId, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            // Need to be authorized first
			let caller = ensure_signed(origin)?;
			ensure!(<Allowances<T>>::exists((ico_id, from.clone(), caller.clone())), "Need to be approved first.");
			let allowance = Self::allowance((ico_id, from.clone(), caller.clone()));
			ensure!(allowance >= value, "Not enough allowance.");

			let new_allowance = allowance.checked_sub(&value).ok_or("underflow in subtracting allowance.")?;
			<Allowances<T>>::insert((ico_id, from.clone(), caller.clone()), new_allowance);

            Self::deposit_event(RawEvent::Approval(from.clone(), caller.clone(), value));

			Self::transfer_impl(ico_id, from, to, value)
        }
	}
}

decl_event!(
	pub enum Event<T> where
	    AccountId = <T as system::Trait>::AccountId,
		Balance = <T as self::Trait>::TokenBalance
	{
		Transfer(AccountId, AccountId, Balance),
		Approval(AccountId, AccountId, Balance),
	}
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    pub fn create_token(sender: T::AccountId, ico_id: u32, name: Vec<u8>, symbol: Vec<u8>, total_supply: T::TokenBalance, decimal: u32) -> Result {
        let t = Token{
            name,
            symbol,
            total_supply,
            decimal,
        };

        <Balances<T>>::insert((ico_id, sender.clone()), total_supply);
        <Tokens<T>>::insert(ico_id, t);
        <Owners<T>>::insert(ico_id, sender);

        Ok(())
    }

    /// internal transfer function
    pub fn transfer_impl(
        ico_id: u32,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> Result {
        ensure!(<Balances<T>>::exists((ico_id, from.clone())), "Account does not own this token");
        let balance_from = Self::balance_of((ico_id, from.clone()));
        ensure!(balance_from >= value, "Not enough balance.");

        // update the balances
        let new_balance_from = balance_from.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of((ico_id, to.clone()));
        let new_balance_to = balance_to.checked_add(&value).ok_or("overflow in adding balance")?;

        <Balances<T>>::insert((ico_id, from.clone()), new_balance_from);
        <Balances<T>>::insert((ico_id, to.clone()), new_balance_to);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }

    pub fn lock(ico_id: u32, from: T::AccountId, value: T::TokenBalance) -> Result {
        ensure!(<Balances<T>>::exists((ico_id, from.clone())), "This account does not own this token");

        let balance_from = Self::balance_of((ico_id, from.clone()));
        ensure!(balance_from > value, "Not enough balance.");
        let updated_balance_from = balance_from.checked_sub(&value).ok_or("overflow in subtracting balance")?;
        let total_lock = Self::total_locked(ico_id);
        let updated_total_lock = total_lock.checked_add(&value).ok_or("overflow in adding deposit")?;

        <Balances<T>>::insert((ico_id, from.clone()), updated_balance_from);
  
        <LockedTokens<T>>::insert((ico_id, from), value);
        <TotalLocked<T>>::insert(ico_id, updated_total_lock);

        Ok(())
    }

    pub fn unlock(ico_id: u32, to: T::AccountId, value: Option<T::TokenBalance>) -> Result {
        let balance_to = Self::balance_of((ico_id, to.clone()));
        let tokens = Self::total_locked(ico_id);
        let v = value.unwrap_or(Self::locked_tokens((ico_id, to.clone())));

        let updated_balance_to = balance_to.checked_add(&v).ok_or("overflow in adding balance")?;
        let updated_tokens = tokens.checked_sub(&v).ok_or("overflow in subtracting deposit")?;

        <LockedTokens<T>>::insert((ico_id, to.clone()), T::TokenBalance::sa(0));
        <Balances<T>>::insert((ico_id, to), updated_balance_to);
        <TotalLocked<T>>::insert(ico_id, updated_tokens);

        Ok(())
    }
}