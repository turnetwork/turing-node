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
		IsController get(is_controller): map T::AccountId => bool;

		// Mapping from tokenHolder to balance.
		Balances get(balance_of): map T::AccountId => T::TokenBalance;

		// list of controllers
		Controllers get(controllers): map u32 => T::AccountId;
		ControllersCount get(controllers_count): u32;

		// operator	=> token_holder	
		AuthorizedOperator get(authorized_operator): map (T::AccountId, T::AccountId) => bool;

		// optional
		IsOperatorFor get(is_operator_for): map (T::AccountId, T::AccountId) => Option<bool>;
		// TODO: certificate
		
		// ---ERC777 end---

		// ---ERC1410 begin---
		// List of partitions.
		TotalPartitions get(total_partitions): map u32 => Vec<u8>;
		PartitionsCount get(partitions_count): u32;

		// Mapping from partition to global balance of corresponding partition.
		ToTalSupplyByPartition get(total_supply_by_partition): map Vec<u8> => T::TokenBalance;

		// Mapping from tokenHolder to their partitions.
		PartitionsOf get(partitions_of): map T::AccountId => Vec<u8>;

		// Mapping from (tokenHolder, partition) to balance of corresponding partition.
		BalancesPartition get(balance_of_by_partition): map (Vec<u8>, T::AccountId) => T::TokenBalance;
		
		// Mapping from tokenHolder to their default partitions (for ERC777 and ERC20 compatibility).
		DefaultPartitionsOf get(default_partitions_of): map T::AccountId => Vec<u8>;

		// List of token default partitions (for ERC20 compatibility).
		TokenDefaultPartitions get(token_default_partitions): Vec<u8>;
		
		// Mapping from (tokenHolder, partition, operator) to 'approved for partition' status. [TOKEN-HOLDER-SPECIFIC]
		AuthorizedOperatorByPartition get(authorized_operator_by_partition): map (T::AccountId, Vec<u8>, T::AccountId) => bool;
		
		// Mapping from partition to controllers_id for the partition. [NOT TOKEN-HOLDER-SPECIFIC]
		ControllersByPartition get(controllers_by_partition): map Vec<u8> => u32;
		
		// Mapping from (partition, operator) to PartitionController status. [NOT TOKEN-HOLDER-SPECIFIC]
		IsControllerByPartition get(is_controller_by_partition): map (Vec<u8>, T::AccountId) => bool;
		
		// Mapping from (partition, operator, token_holder)
		IsOperatorForPartition get(is_operator_for_partition): map (Vec<u8>, T::AccountId, T::AccountId) => Option<bool>;
		// ---ERC1410 end---

		// ---ERC1400 begin---
		// TODO: what is this?
		Granularity get(granularity): u128;

		Documents get(get_document): map Vec<u8> => Doc<T::Hash>;
		Issuable get(is_issuable): bool = true;
		
		Operator get(operator): map (T::AccountId, T::AccountId) => bool;
		OperatorForPartition get(operator_for_partition): map (Vec<u8>, T::AccountId, T::AccountId) => bool;
		// ---ERC1400 end---
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

		// ---ERC777 begin---
		fn authorize_operator(origin, operator: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			<AuthorizedOperator<T>>::insert((operator.clone(), sender.clone()), true);
			Self::deposit_event(RawEvent::AuthorizedOperator(operator, sender));
			Ok(())
		}

		fn revoke_operator(origin, operator: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			<AuthorizedOperator<T>>::insert((operator.clone(), sender.clone()), false);
			Self::deposit_event(RawEvent::RevokedOperator(operator, sender));
			Ok(())
		}

		fn check_operator_for(operator: T::AccountId, token_holder: T::AccountId) -> Result {
			let result = Self::is_operator_for((operator.clone(), token_holder.clone()));
			if result == None {
				let is_for = Self::_is_operator_for(operator.clone(), token_holder.clone());
				<IsOperatorFor<T>>::insert((operator, token_holder), Some(is_for));
			}
			Ok(())
		}
		
		// TODO: isValidCertificate(data)?
		fn transfer_with_data(origin, to: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result{
			let sender = ensure_signed(origin)?;
			Self::_transfer_with_data("", sender.clone(), sender, to, value, data, "", true);
			Ok(())
		}

		// TODO: isValidCertificate(data)?
		fn transfer_from_with_data(
			origin, 
			from: T::AccountId, 
			to: T::AccountId, 
			#[compact] value: T::TokenBalance, 
			data: Vec<u8>,
			operator_data: Vec<u8>
		) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(_is_operator_for(sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");
			Self::_transfer_with_data("", sender, from, to, value, data, operator_data, true);
			Ok(())
		}

		// TODO: isValidCertificate(data)?
		fn redeem(origin, value: TokenBalance, data: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_redeem("", sender.clone(), sender, value, data)
		}

		// TODO: isValidCertificate(data)?
		fn redeem_from(origin, token_holder: T::AccountId, value: T::TokenBalance, data: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(_is_operator_for(sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");
			Self::_redeem("", sender, token_holder, value, data)
		}

		// ---ERC777 end---

		// ---ERC1410 begin---
		// TODO: isValidCertificate(data)?
		fn transfer_by_partition(
			origin,
			partition: Vec<u8>, 
			to: T::AccountId, 
			#[compact] value: T::TokenBalance, 
			data: Vec<u8>
		) -> Result {
			let sender = ensure_signed(origin)?;
			Self::_transfer_by_partition(partition, sender.clone(), sender, to, value, data)
		}

		// TODO: isValidCertificate(operator_data)?
		fn operator_transfer_by_partition(
			origin,
			partition: Vec<u8>, 
			from: T::AccountId, 
			to: T::AccountId, 
			data: Vec<u8>,
			operator_data: Vec<u8>
		) -> Result {
			let sender = ensure_signed(origin)?;
			ensure!(_is_operator_for_partition(partition.clone(), sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");

			Self::_transfer_by_partition(partition, sender, from, to, value, data, operator_data)
		}

		fn set_default_partitons(origin, partitons: Vec<u8>) -> Result {
			let sender = ensure_signed(origin)?;
			<DefaultPartitionsOf<T>>::insert(sender, partitons);
			Ok(())
		}

		fn authorize_operator_by_partition(origin, partition: Vec<u8>, operator: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			<AuthorizedOperatorByPartition<T>>::insert((sender.clone(), partition.clone(), operator.clone()), true);
			Self::deposit_event(RawEvent::AuthorizedOperatorByPartition(partition, operator, sender));
			Ok(())
		}

		fn revoke_operator_by_partition(origin, partition: Vec<u8>, operator: T::AccountId) -> Result {
			let sender = ensure_signed(origin)?;
			<AuthorizedOperatorByPartition<T>>::insert((sender.clone(), partition.clone(), operator.clone()), false);
			Self::deposit_event(RawEvent::RevokedOperatorByPartition(partition, operator, sender));
			Ok(())
		}

		fn check_operator_for_partition(partition: Vec<u8>, operator: T::AccountId, token_holder: T::AccountId) -> Result {
			let result = Self::is_operator_for((partition.clone(), operator.clone(), token_holder.clone()));
			if result == None {
				let is_for = Self::_is_operator_for_partition(partition.clone(), operator.clone(), token_holder.clone());
				<IsOperatorForPartition<T>>::insert((partition, operator, token_holder), Some(is_for));
			}
			Ok(())
		}

		// ---ERC1410 end---

		// ---ERC1400 begin---
		fn set_document(name: Vec<u8>, uri: Vec<u8>, document_hash: T::Hash) -> Result {
			let d = Doc{
				docURI: uri,
				docHash: document_hash,
			};
			<Documents<T>>::insert(name, d);
			Self::deposit_event(RawEvent::Document(name, uri, document_hash));
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

		fn issue(token_holder: T::AccountId, value: T::TokenBalance, data: Vec<u8>) -> Result {
			Ok(())
		}

		fn issue_by_partition(partition: Vec<u8>, token_holder: T::AccountId, value: T::TokenBalance, data: Vec<u8>) -> Result {
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

		// TODO: finish this fn
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

/**
 * Reason codes - ERC1066
 *
 * To improve the token holder experience, canTransfer MUST return a reason byte code
 * on success or failure based on the EIP-1066 application-specific status codes specified below.
 * An implementation can also return arbitrary data as a bytes32 to provide additional
 * information not captured by the reason code.
 *
 * Code	Reason
 * 0xA0	Transfer Verified - Unrestricted
 * 0xA1	Transfer Verified - On-Chain approval for restricted token
 * 0xA2	Transfer Verified - Off-Chain approval for restricted token
 * 0xA3	Transfer Blocked - Sender lockup period not ended
 * 0xA4	Transfer Blocked - Sender balance insufficient
 * 0xA5	Transfer Blocked - Sender not eligible
 * 0xA6	Transfer Blocked - Receiver not eligible
 * 0xA7	Transfer Blocked - Identity restriction
 * 0xA8	Transfer Blocked - Token restriction
 * 0xA9	Transfer Blocked - Token granularity
 */
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

		// operator, from, to, value, data, operator_data
		TransferWithData(
			AccountId,
			AccountId,
			AccountId,
			Balance,
			Vec<u8>,
			Vec<u8>
		), 

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

	// ---ERC777 begin---
	fn _is_operator_for(operator: T::AccountId, token_holder: T::AccountId) -> bool {
		operator == token_holder
		|| authorized_operator((operator, token_holder))
		|| (is_controllable() && is_controller(operator))
	}

	fn _transfer_with_data(
		partition: Vec<u8>,
		operator: T::AccountId,
		from: T::AccountId,
		to: T::AccountId,
		value: T::TokenBalance,
		data: Vec<u8>,
		operator_data: Vec<u8>,
		prevent_locking: bool
	) -> Result {
		ensure!(<Balances<T>>::exists(from.clone()), "Account does not own this token");
		ensure!(_is_multiple(value.clone()), "A9: Transfer Blocked - Token granularity");
		// ensure!(to.clone() != T::AccountId::sa(0), "A6: Transfer Blocked - Receiver not eligible");
		ensure!(Self::balance_of(from.clone()) >= value.clone(), "A4: Transfer Blocked - Sender balance insufficient");

		// TODO: call_sender
		// _call_sender(partition.clone(), operator.clone(), from.clone(), to.clone(), value.clone(), data.clone(), operator_data.clone());

		// update the balances
        let balance_from = Self::balance_of(from.clone());
        let new_balance_from = balance_from.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of(to.clone());
        let new_balance_to = balance_to.checked_add(&value).ok_or("overflow in adding balance")?;

        <Balances<T>>::insert(from.clone(), new_balance_from);
        <Balances<T>>::insert(to.clone(), new_balance_to);

		// TODO: call_recipient

		Self::deposit_event(RawEvent::TransferWithData(operator, from, to, value, data, operator_data));
	}

	fn _redeem(
		partition: Vec<u8>, 
		operator: T::AccountId, 
		from: T::AccountId, 
		value: T::TokenBalance, 
		data: Vec<u8>
	) -> Result {
		ensure!(_is_multiple(value.clone), "A9: Transfer Blocked - Token granularity");
		//  "A5: Transfer Blocked - Sender not eligible" ?

		let balance_from = Self::balance_of(from.clone());
		ensure!(balance_from >= value.clone(), "A4: Transfer Blocked - Sender balance insufficient");

		// TODO: callsender

        let new_balance_from = balance_from.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        let total_supply = Self::total_supply();
        let new_total_supply = total_supply.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        <Balances<T>>::insert(from.clone(), new_balance_from);
		<TotalSupply<T>>::put(new_total_supply);

		Self::deposit_event(RawEvent::Redeemed(operator, from, value, data));
		Ok(())
	}
	// ---ERC777 end---

	// ---ERC1410 begin---
	fn _transfer_by_partition(
		partition: Vec<u8>,
		operator: T::AccountId,
		from: T::AccountId,
		to: T::AccountId,
		value: T::TokenBalance,
		data: Vec<u8>,
		operator_data: Vec<u8> 
	) -> Result {
		ensure!(balance_of_by_partition((from.clone(), partition.clone())) >= value.clone(), "A4: Transfer Blocked - Sender balance insufficient"); // ensure enough funds
		let mut to_partition = partiton.clone();
		if operator_data.len() != 0 && data.len() != 0 {
			to_partition = Self::_get_destination_partition(partiton.clone(), data.clone());
		}

		Self::_remove_token_from_partition(from.clone(),  partition.clone(), value.clone());
		Self::_transfer_with_data(
			partition.clone(), 
			operator.clone(), 
			from.clone(), 
			to.clone(), 
			value.clone(), 
			data.clone(), 
			operator_data.clone(),
			true
		);
		Self::_add_token_to_partition(
			to.clone(),
			to_partition.clone(),
			value.clone()
		);

		Self::deposit_event(RawEvent::TransferByPartition(
			partition.clone(), 
			operator.clone(),
			from.clone(),
			to.clone(),
			value.clone(),
			data.clone(),
			operator_data.clone()
		));

		if to_partition.clone() != partition.clone() {
			Self::deposit_event(RawEvent::ChangedPartition(partition, to_partition, value));
		}
		Ok(())
	}

	fn _is_operator_for_partition(partition: Vec<u8>, operator: T::AccountId, token_holder: T::AccountId) -> bool {
		(_is_operator_for(operator.clone(), token_holder.clone()))
		|| authorize_operator_by_partition((token_holder.clone(), partition.clone(), operator.clone()))
		|| (is_controllable() && is_controller_by_partition((partition, operator)))
	}

	fn _remove_token_from_partition(
		from: T::AccountId,
		partition: Vec<u8>,
		value: T::TokenBalance
	) -> Result {
		let balance_of_by_partition = Self::balance_of_by_partition((from.clone(), partition.clone()));
		let new_balance_of_by_partition = balance_of_by_partition.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        let total_supply_by_partition = Self::total_supply_by_partition();
        let new_total_supply_by_partition = total_supply_by_partition.checked_sub(&value).ok_or("underflow in subtracting balance")?;
        
		<BalancesPartition<T>>::insert(from.clone(), new_balance_of_by_partition);
		<ToTalSupplyByPartition<T>>::insert(partiton.clone(), new_total_supply_by_partition);

		if balance_of_by_partition((from.clone(), partition.clone())) == 0 {
			for i in [0..]
		}
	}

	// ---ERC1410 end---
}