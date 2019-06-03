/// A simple implementation of the ERC20_multi
// refer to https://github.com/substrate-developer-hub/substrate-erc20-multi

use rstd::prelude::Vec;
use support::{ensure, Parameter, StorageValue, StorageMap, decl_module, decl_storage, decl_event, dispatch::Result};
use parity_codec::{Codec, Encode, Decode};
use runtime_primitives::traits::{
    SimpleArithmetic, Member, CheckedAdd, CheckedSub
};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type TokenBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + From<Self::BlockNumber>;
}

// struct to store the token details
#[derive(Encode, Decode, Default, Clone, PartialEq, Debug)]
pub struct Erc20Token<T> {
    name: Vec<u8>,
    symbol: Vec<u8>,
    total_supply: T,
}

// This module's storage items.
decl_storage! {
  trait Store for Module<T: Trait> as Erc20Multi {
      TokenId get(token_id): u32;
      // details of the token corresponding to a token id
      Tokens get(token_details): map u32 => Erc20Token<T::TokenBalance>;
      // balances mapping for an account and token
      BalanceOf get(balance_of): map (u32, T::AccountId) => T::TokenBalance;
      // allowance for an account and token
      Allowance get(allowance): map (u32, T::AccountId, T::AccountId) => T::TokenBalance;
  }
}

decl_event!(
    pub enum Event<T> where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as self::Trait>::TokenBalance
    {
        // event for transfer of tokens
        // tokenid, from, to, value
        Transfer(u32, AccountId, AccountId, Balance),
        // event when an approval is made
        // tokenid, owner, spender, value
        Approval(u32, AccountId, AccountId, Balance),
    }
);

// public interface for this runtime module
decl_module! {
  pub struct Module<T: Trait> for enum Call where origin: T::Origin {
      // initialize the default event for this module
      fn deposit_event<T>() = default;

      // initializes a new token
      // generates an integer token_id so that all tokens are unique
      // takes a name, ticker, total supply for the token
      // makes the initiating account the owner of the token
      // the balance of the owner is set to total supply
      fn init(origin, name: Vec<u8>, symbol: Vec<u8>, total_supply: T::TokenBalance) -> Result {
          let sender = ensure_signed(origin)?;

          // checking max size for name and ticker
          // byte arrays (vecs) with no max size should be avoided
          ensure!(name.len() <= 64, "token name cannot exceed 64 bytes");
          ensure!(symbol.len() <= 32, "token ticker cannot exceed 32 bytes");

          let token_id = Self::token_id();
          let next_token_id = token_id.checked_add(1).ok_or("overflow in calculating next token id")?;
          <TokenId<T>>::put(next_token_id);

          let token = Erc20Token {
              name,
              symbol,
              total_supply,
          };

          <Tokens<T>>::insert(token_id, token);
          <BalanceOf<T>>::insert((token_id, sender), total_supply);

          Ok(())
      }

      // transfer tokens from one account to another
      // origin is assumed as sender
      fn transfer(_origin, token_id: u32, to: T::AccountId, value: T::TokenBalance) -> Result {
          let sender = ensure_signed(_origin)?;
          Self::_transfer(token_id, sender, to, value)
      }

      // approve token transfer from one account to another
      // once this is done, transfer_from can be called with corresponding values
      fn approve(_origin, token_id: u32, spender: T::AccountId, value: T::TokenBalance) -> Result {
          let sender = ensure_signed(_origin)?;
          ensure!(<BalanceOf<T>>::exists((token_id, sender.clone())), "Account does not own this token");

          ensure!(spender != sender, "Owner is implicitly approved");

          let allowance = Self::allowance((token_id, sender.clone(), spender.clone()));
          let updated_allowance = allowance.checked_add(&value).ok_or("overflow in calculating allowance")?;
          <Allowance<T>>::insert((token_id, sender.clone(), spender.clone()), updated_allowance);

          Self::deposit_event(RawEvent::Approval(token_id, sender.clone(), spender.clone(), value));

          Ok(())
      }

      // the ERC20 standard transfer_from function
      // implemented in the open-zeppelin way - increase/decrease allownace
      // if approved, transfer from an account to another account without owner's signature
      pub fn transfer_from(_origin, token_id: u32, from: T::AccountId, to: T::AccountId, value: T::TokenBalance) -> Result {
        // Need to be authorized first
	    let caller = ensure_signed(_origin)?;

        ensure!(<Allowance<T>>::exists((token_id, from.clone(), caller.clone())), "Allowance does not exist.");
        let allowance = Self::allowance((token_id, from.clone(), caller.clone()));
        ensure!(allowance >= value, "Not enough allowance.");

        // using checked_sub (safe math) to avoid overflow
        let updated_allowance = allowance.checked_sub(&value).ok_or("overflow in calculating allowance")?;
        <Allowance<T>>::insert((token_id, from.clone(), caller.clone()), updated_allowance);

        Self::deposit_event(RawEvent::Approval(token_id, from.clone(), caller.clone(), value));
        Self::_transfer(token_id, from, to, value)
      }
  }
}

// implementation of mudule
// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    // the ERC20 standard transfer function
    // internal
    fn _transfer(
        token_id: u32,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
    ) -> Result {
        ensure!(<BalanceOf<T>>::exists((token_id, from.clone())), "Account does not own this token");
        let sender_balance = Self::balance_of((token_id, from.clone()));
        ensure!(sender_balance >= value, "Not enough balance.");

        let updated_from_balance = sender_balance.checked_sub(&value).ok_or("overflow in calculating balance")?;
        let receiver_balance = Self::balance_of((token_id, to.clone()));
        let updated_to_balance = receiver_balance.checked_add(&value).ok_or("overflow in calculating balance")?;

        // reduce sender's balance
        <BalanceOf<T>>::insert((token_id, from.clone()), updated_from_balance);

        // increase receiver's balance
        <BalanceOf<T>>::insert((token_id, to.clone()), updated_to_balance);

        Self::deposit_event(RawEvent::Transfer(token_id, from, to, value));
        Ok(())
    }
}
