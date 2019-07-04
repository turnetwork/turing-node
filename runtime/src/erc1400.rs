/// A simple implementation of the ERC1400, reference: https://github.com/ethereum/EIPs/issues/1411

use rstd::prelude::Vec;
use support::{ensure, Parameter, StorageMap, decl_module, decl_storage, decl_event, dispatch::Result};
use system::ensure_signed;
use parity_codec::Codec;
use runtime_primitives::traits::{As, SimpleArithmetic, Member, CheckedAdd, CheckedSub};

#[cfg(feature = "std")]
use runtime_io::with_storage;

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
	type TokenBalance: Parameter + Member + SimpleArithmetic + Codec + Default + Copy + As<usize> + As<u64>;
}

struct Doc <Hash>{
	docURI: Vec<u8>,
	docHash: Hash,
}

// This module's storage items.
decl_storage! {
	trait Store for Module<T: Trait> as ERC1400 {
		// use config() to set the data in the genesis config
		Owner get(owner) config(): T::AccountId;

		// ---ERC777 begin---
		Name get(name) config(): Vec<u8>;
		Symbol get(symbol) config(): Vec<u8>;
		Decimal get(decimal) : u16 = 18;
		Totalsupply get(total_supply) config() : T::TokenBalance;
		Controllable get(is_controllable): bool = true;

		// Mapping from tokenHolder to balance.
		Balances get(balance_of): map T::AccountId => T::TokenBalance;

		// list of controllers
		Controllers get(controllers): map u32 => T::AccountId;
		ControllersCount get(controllers_count): u32;

		// token_holder => operator		
		AuthorizeOperator get(authorize_operator): map T::AccountId => T::AccountId;
		
		// ---ERC777 end---

		// ERC1400
		// TODO: what is this?
		Granularity get(granularity): u128;

		// List of partitions.
		TotalPartitions get(total_partitions): map u32 => Vec<u8>;
		PartitionsCount get(partitions_count): u32;

		Partitions get(partition_of): map T::AccountId => Vec<u8>;

		Documents get(get_document): map Vec<u8> => Doc<T::Hash>;

		BalancesPartition get(balance_of_by_partition): map (Vec<u8>, T::AccountId) => T::TokenBalance;
		
		RevokeOperator get(revoke_operator): T::AccountId;
		Operator get(operator): map (T::AccountId, T::AccountId) => bool;
		OperatorForPartition get(operator_for_partition): map (Vec<u8>, T::AccountId, T::AccountId) => bool;
		Issuable get(is_issuable): bool = true;
	}

	add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageOverlay, _: &mut runtime_primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            with_storage(storage, || {
                <Balances<T>>::insert(config.owner.clone(), config.total_supply.clone());
            })
        })
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

		// ERC1400 begin
		fn set_document(name: Vec<u8>, uri: Vec<u8>, document_hash: T::Hash) -> Result {
			Ok(())
		}

		fn transfer_with_data(origin, to: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result{
			Ok(())
		}

		fn transfer_from_with_data(
			origin, 
			from: T::AccountId, 
			to: T::AccountId, 
			#[compact] value: T::TokenBalance, 
			data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn transfer_by_partition(
			partition: Vec<u8>, 
			to: T::AccountId, 
			#[compact] value: T::TokenBalance, 
			data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn operator_transfer_by_partition(
			partition: Vec<u8>, 
			from: T::AccountId, 
			to: T::AccountId, 
			data: Vec<u8>,
			operator_data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn controller_transfer(
			from: T::AccountId, 
			to: T::AccountId, 
			#[compact] value: T::TokenBalance,
			data: Vec<u8>,
			operator_data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn controller_redeem(
			token_holder: T::AccountId,
			#[compact] value: T::TokenBalance,
			data: Vec<u8>,
			operator_data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn authorize_operator_by_partition(partition: Vec<u8>, operator: T::AccountId) -> Result {
			Ok(())
		}

		fn revoke_operator_by_partition(partition: Vec<u8>, operator: T::AccountId) -> Result {
			Ok(())
		}

		fn issue(token_holder: T::AccountId, value: T::TokenBalance, data: Vec<u8>) -> Result {
			Ok(())
		}

		fn issue_by_partition(partition: Vec<u8>, token_holder: T::AccountId, value: T::TokenBalance, data: Vec<u8>) -> Result {
			Ok(())
		}

		fn redeem(value: TokenBalance, data: Vec<u8>) -> Result {
			Ok(())
		}

		fn redeem_from(token_holder: T::AccountId, value: T::TokenBalance, data: Vec<u8>) -> Result {
			Ok(())
		}

		fn redeem_by_partition(partition: Vec<u8>, value: T::TokenBalance, data: Vec<u8>) -> Result {
			Ok(())
		}

		fn operator_redeem_by_partition(
			partition: Vec<u8>, 
			token_holder: T::AccountId,
			value: T::TokenBalance, 
			operator_data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn can_transfer(
			to: T::AccountId,
			value: T::TokenBalance,
			data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn can_transfer_from(
			from: T::AccountId,
			to: T::AccountId,
			value: TokenBalance,
			data: Vec<u8>
		) -> Result {
			Ok(())
		}

		fn can_transfer_by_partition(
			from: T::AccountId,
			to: T::AccountId,
			partition: Vec<u8>,
			value: T::TokenBalance,
			data: Vec<u8>
		) -> Result {
			Ok(())
		}
		// ERC1400 end
	}
}

decl_event!(
	pub enum Event<T> where
	    AccountId = <T as system::Trait>::AccountId,
		Balance = <T as self::Trait>::TokenBalance,
		string = Vec<u8>,
		Hash = <T:: system::Trait>::Hash,
	{
		// from, to, value
		Transfer(AccountId, AccountId, Balance),
		Approval(AccountId, AccountId, Balance),

		// controller, from, to, value, data, operatorData
		ControllerTransfer(AccountId, AccountId, AccountId, TokenBalance, string, string),
		// controller, token_holder, value, data, operatorData
		ControllerRedemption(AccountId, AccountId, TokenBalance, string, string),
		
		// name, uri, document_hash
		Document(string, string, Hash),

		// fromPartition, operator, from, to, value, data, operatorData
		TransferByPartition(
			string,
			AccountId,
			AccountId,
			AccountId,
			TokenBalance,
			string,
			string
		),

		// fromPartition, toPartition, value
		ChangedPartition(string, string, TokenBalance),
		
		// Operator Events
		// address indexed _operator, address indexed _tokenHolder
  		AuthorizedOperator(AccountId, AccountId),
		// address indexed _operator, address indexed _tokenHolder
   		RevokedOperator(AccountId, AccountId),
		// bytes32 indexed _partition, address indexed _operator, address indexed _tokenHolder
  		AuthorizedOperatorByPartition(string, AccountId, AccountId),
		// bytes32 indexed _partition, address indexed _operator, address indexed _tokenHolder
  		RevokedOperatorByPartition(string, AccountId, AccountId),

  		// Issuance / Redemption Events
		// address indexed _operator, address indexed _to, uint256 _value, bytes _data
  		Issued(AccountId, AccountId, TokenBalance, string),
		// address indexed _operator, address indexed _from, uint256 _value, bytes _data
  		Redeemed(AccountId, AccountId, TokenBalance, string),
		// bytes32 indexed _partition, address indexed _operator, address indexed _to, uint256 _value, bytes _data, bytes _operatorData
  		IssuedByPartition(string, AccountId, AccountId, TokenBalance, string, string),
		// bytes32 indexed _partition, address indexed _operator, address indexed _from, uint256 _value, bytes _operatorData
  		RedeemedByPartition(string, AccountId, AccountId, TokenBalance, string),

	}
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
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

	
}