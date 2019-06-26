/// A special implementation of lockable ERC20 interface

use rstd::prelude::Vec;
use support::{ensure, Parameter, StorageValue, StorageMap, decl_module, decl_storage, decl_event, dispatch::Result};
use system::ensure_signed;
use parity_codec::Codec;
use runtime_primitives::traits::{As, SimpleArithmetic, Member, CheckedAdd, CheckedSub};

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64>;
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as DaoToken {
        Init get(is_init): bool;
		Balances get(balance_of): map T::AccountId => T::TokenBalance;
		Allowances get(allowance): map (T::AccountId, T::AccountId) => T::TokenBalance;

		Totalsupply get(total_supply) config(): T::TokenBalance;

		// Optional
		Name get(name) config(): Vec<u8>;
		Symbol get(symbol) config(): Vec<u8>;
		DecimalPlaces get(decimal) config(): u32;

		// special interface
		LockedDeposits get(locked_deposits): map u64 => T::TokenBalance;
	}
}

decl_module! {
	/// The module declaration.
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Initializing events
		// this is needed only if you are using events in your module
		fn deposit_event<T>() = default;

		/// Transfers token from the sender to the `to` address.
		fn transfer(origin, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            Self::transfer_impl(sender, to, value)
        }

		/// Approve the passed address to spend the specified amount of tokens on the behalf of the message's sender.
        fn approve(origin, spender: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let owner = ensure_signed(origin)?;
			ensure!(<Balances<T>>::exists(&owner), "Account does not own this token");

			ensure!(spender != owner, "Owner is implicitly approved");

            let allowance = Self::allowance((owner.clone(), spender.clone()));
			let new_allowance = allowance.checked_add(&value).ok_or("overflow in adding allowance")?;

			<Allowances<T>>::insert((owner.clone(), spender.clone()), new_allowance);

			Self::deposit_event(RawEvent::Approval(owner, spender, value));
            Ok(())
        }

        /// Transfer tokens from one address to another by allowance
        fn transfer_from(origin, from: T::AccountId, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            // Need to be authorized first
			let caller = ensure_signed(origin)?;
			ensure!(<Allowances<T>>::exists((from.clone(), caller.clone())), "Need to be approved first.");
			let allowance = Self::allowance((from.clone(), caller.clone()));
			ensure!(allowance >= value, "Not enough allowance.");

			let new_allowance = allowance.checked_sub(&value).ok_or("underflow in subtracting allowance.")?;
			<Allowances<T>>::insert((from.clone(), caller.clone()), new_allowance);

            Self::deposit_event(RawEvent::Approval(from.clone(), caller.clone(), value));

			Self::transfer_impl(from, to, value)
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
    pub fn init(sender: T::AccountId) -> Result {
        ensure!(Self::is_init() == false, "Token already initialized.");

        <Balances<T>>::insert(sender, Self::total_supply());
        <Init<T>>::put(true);

        Ok(())
    }

    /// internal transfer function
    fn transfer_impl(
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> Result {
        ensure!(<Balances<T>>::exists(from.clone()), "Account does not own this token");
        let balance_from = Self::balance_of(from.clone());
        ensure!(balance_from >= value, "Not enough balance.");

        // update the balances
        let new_balance_from = balance_from.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of(to.clone());
        let new_balance_to = balance_to.checked_add(&value).ok_or("overflow in adding balance")?;

        <Balances<T>>::insert(from.clone(), new_balance_from);
        <Balances<T>>::insert(to.clone(), new_balance_to);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }

    pub fn lock(from: T::AccountId, value: T::TokenBalance, proposal_id: u64) -> Result {
        ensure!(<Balances<T>>::exists(from.clone()), "This account does not own this token");

        let balance_from = Self::balance_of(from.clone());
        ensure!(balance_from > value, "Not enough balance.");
        let updated_balance_from = balance_from.checked_sub(&value).ok_or("overflow in subtracting balance")?;
        let deposit = Self::locked_deposits(proposal_id);
        let updated_deposit = deposit.checked_add(&value).ok_or("overflow in adding deposit")?;

        <Balances<T>>::insert(from, updated_balance_from);

        <LockedDeposits<T>>::insert(proposal_id, updated_deposit);

        Ok(())
    }

    pub fn unlock(to: T::AccountId, value: T::TokenBalance, proposal_id: u64) -> Result {
        let balance_to = Self::balance_of(to.clone());
        let updated_balance_to = balance_to.checked_add(&value).ok_or("overflow in adding balance")?;
        let deposit = Self::locked_deposits(proposal_id.clone());
        let updated_deposit = deposit.checked_sub(&value).ok_or("overflow in subtracting deposit")?;

        <Balances<T>>::insert(to, updated_balance_to);

        <LockedDeposits<T>>::insert(proposal_id, updated_deposit);

        Ok(())
    }
}