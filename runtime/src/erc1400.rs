/// A simple implementation of the ERC1400, reference: https://github.com/ethereum/EIPs/issues/1411
use parity_codec::{Codec, Decode, Encode};
use rstd::prelude::*;
use runtime_primitives::traits::{As, CheckedAdd, CheckedSub, Member, SimpleArithmetic, Hash};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter, StorageMap, StorageValue,
};
use system::ensure_signed;

#[cfg(feature = "std")]
use runtime_io::with_storage;

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

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(Encode, Decode, Default, Clone, PartialEq)]
struct Doc<Hash> {
    docURI: Vec<u8>,
    docHash: Hash,
}

pub type Bytes32 = [u8; 32];

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ERC1400 {
        // use config() to set the data in the genesis config
        Owner get(owner) config(): T::AccountId;

        // ---ERC777 begin---
        Name get(name) config(): Vec<u8>;
        Symbol get(symbol) config(): Vec<u8>;
        Decimal get(decimal) : u16 = 18;
        TotalSupply get(total_supply) config() : T::TokenBalance;
        Controllable get(is_controllable): bool = true;
        IsController get(is_controller): map T::AccountId => bool;

        // Mapping from tokenHolder to balance.
        Balances get(balance_of): map T::AccountId => T::TokenBalance;

        // list of controllers
        Controllers get(controllers): Vec<T::AccountId>;

        // operator	=> token_holder
        AuthorizedOperator get(authorized_operator): map (T::AccountId, T::AccountId) => bool;

        // optional
        IsOperatorFor get(is_operator_for): map (T::AccountId, T::AccountId) => Option<bool>;
        // TODO: certificate

        // ---ERC777 end---

        // ---ERC1410 begin---
        // List of partitions.
        TotalPartitions get(total_partitions): map u64 => Bytes32;
        TotalPartitionsCount get(total_partitions_count): u64;

        // Mapping from partition to global balance of corresponding partition.
        ToTalSupplyByPartition get(total_supply_by_partition): map Bytes32 => T::TokenBalance;

        // Mapping from tokenHolder to their partitions.
        PartitionsOf get(partitions_of): map (T::AccountId, u64) => Bytes32;
        PartitionsOfCount get(partitions_of_count): u64;

        // Mapping from (tokenHolder, partition) to balance of corresponding partition.
        BalancesPartition get(balance_of_by_partition): map (T::AccountId, Bytes32) => T::TokenBalance;

        // Mapping from tokenHolder to their default partitions (for ERC777 and ERC20 compatibility).
        DefaultPartitionsOf get(default_partitions_of): map T::AccountId => Vec<Bytes32>;

        // List of token default partitions (for ERC20 compatibility).
        TokenDefaultPartitions get(token_default_partitions): Vec<Bytes32>;

        // Mapping from (tokenHolder, partition, operator) to 'approved for partition' status. [TOKEN-HOLDER-SPECIFIC]
        AuthorizedOperatorByPartition get(authorized_operator_by_partition): map (T::AccountId, Vec<Bytes32>, T::AccountId) => bool;

        // Mapping from partition to controllers for the partition. [NOT TOKEN-HOLDER-SPECIFIC]
        ControllersByPartition get(controllers_by_partition): map Bytes32 => Vec<T::AccountId>;

        // Mapping from (partition, operator) to PartitionController status. [NOT TOKEN-HOLDER-SPECIFIC]
        IsControllerByPartition get(is_controller_by_partition): map (Bytes32, T::AccountId) => bool;

        // Mapping from (partition, operator, token_holder)
        IsOperatorForPartition get(is_operator_for_partition): map (Bytes32, T::AccountId, T::AccountId) => Option<bool>;
        // ---ERC1410 end---

        // ---ERC1400 begin---
        // TODO: what is this?
        Granularity get(granularity): T::TokenBalance;

        Documents get(get_document): map Bytes32 => Doc<T::Hash>;
        Issuable get(is_issuable): bool = true;

        Operator get(operator): map (T::AccountId, T::AccountId) => bool;
        OperatorForPartition get(operator_for_partition): map (Bytes32, T::AccountId, T::AccountId) => bool;
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
            ensure!(Self::_is_operator_for(sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");
            Self::_transfer_with_data("", sender, from, to, value, data, operator_data, true);
            Ok(())
        }

        // TODO: isValidCertificate(data)?
        fn redeem(origin, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            Self::_redeem("".into(), sender.clone(), sender, value, data)
        }

        // TODO: isValidCertificate(data)?
        fn redeem_from(origin, token_holder: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::_is_operator_for(sender.clone(), token_holder.clone()), "A7: Transfer Blocked - Identity restriction");
            Self::_redeem("".into(), sender, token_holder, value, data)
        }

        // ---ERC777 end---

        // ---ERC1410 begin---
		
        // TODO: isValidCertificate(data)?
        /// Transfer tokens from a specific partition.
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
        /// Transfer tokens from a specific partition through an operator.
        fn operator_transfer_by_partition(
            origin,
            partition: Vec<u8>,
            from: T::AccountId,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::_is_operator_for_partition(partition.clone(), sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");

            Self::_transfer_by_partition(partition, sender, from, to, value, data, operator_data)
        }

        /// Set default partitions to transfer from.
           /// Function used for ERC777 and ERC20 backwards compatibility.
        fn set_default_partitons(origin, partitons: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            <DefaultPartitionsOf<T>>::insert(sender, partitons);
            Ok(())
        }

        /// Set 'operator' as an operator for 'msg.sender' for a given partition.
        fn authorize_operator_by_partition(origin, partition: Vec<u8>, operator: T::AccountId) -> Result {
            let sender = ensure_signed(origin)?;
            <AuthorizedOperatorByPartition<T>>::insert((sender.clone(), partition.clone(), operator.clone()), true);
            Self::deposit_event(RawEvent::AuthorizedOperatorByPartition(partition, operator, sender));
            Ok(())
        }

        /// Remove the right of the operator address to be an operator on a given
        /// partition for 'msg.sender' and to transfer and redeem tokens on its behalf.
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
        fn set_document(name: Bytes32, uri: Vec<u8>, document_hash: T::Hash) -> Result {
            let d = Doc{
                docURI: uri,
                docHash: document_hash,
            };
            <Documents<T>>::insert(name, d);
            Self::deposit_event(RawEvent::Document(name, uri, document_hash));
            Ok(())
        }

        // TODO:
        fn controller_transfer(
            from: T::AccountId,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            Ok(())
        }

        // TODO:
        fn controller_redeem(
            token_holder: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            Ok(())
        }

        // TODO:
        fn issue(token_holder: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            Ok(())
        }

        // TODO: isValidCertificate(data)?
        /// Issue tokens from a specific partition.
        fn issue_by_partition(origin, partition: Bytes32, token_holder: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            Self::_issue_by_partition(partition, sender, token_holder, value, data, "".into())
        }

        // TODO: isValidCertificate(data)?
        /// Redeem tokens of a specific partition.
        fn redeem_by_partition(origin, partition: Bytes32, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            Self::_redeem_by_partition(partition, sender.clone(), sender, value, data, "")
        }

        // TODO: isValidCertificate(operatorData)?
        /// Redeem tokens of a specific partition.
        fn operator_redeem_by_partition(
            origin,
            partition: Bytes32,
            token_holder: T::AccountId,
            #[compact] value: T::TokenBalance,
            operator_data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_operator_for_partition((
                partition.clone(),
                sender.clone(),
                token_holder.clone()
                )), "A7: Transfer Blocked - Identity restriction");

            Self::_redeem_by_partition(partition, sender, token_holder, value, data, operator_data)
        }

        // TODO: finish this fn
        fn can_transfer(
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>
        ) -> Result {
            Ok(())
        }

        // TODO: finish this fn
        /// Know the reason on success or failure based on the EIP-1066 application-specific status codes.
        fn can_transfer_by_partition(
            from: T::AccountId,
            to: T::AccountId,
            partition: Vec<u8>,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>
        ) -> Result {
            Ok(())
        }
        // ERC1400 end
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as self::Trait>::TokenBalance,
        string = Vec<u8>,
        Hash = <T as system::Trait>::Hash,
        Bytes32 = Bytes32,
    {
        // from, to, value
        Transfer(AccountId, AccountId, Balance),
        Approval(AccountId, AccountId, Balance),

        // controller, from, to, value, data, operatorData
        ControllerTransfer(AccountId, AccountId, AccountId, Balance, string, string),
        // controller, token_holder, value, data, operatorData
        ControllerRedemption(AccountId, AccountId, Balance, string, string),

        // name, uri, document_hash
        Document(string, string, Hash),

        // operator, from, to, value, data, operator_data
        TransferWithData(AccountId, AccountId, AccountId, Balance, Vec<u8>, Vec<u8>),

        // fromPartition, operator, from, to, value, data, operatorData
        TransferByPartition(
            Bytes32,
            AccountId,
            AccountId,
            AccountId,
            Balance,
            string,
            string
        ),

        // fromPartition, toPartition, value
        ChangedPartition(Bytes32, Bytes32, Balance),

        // Operator Events
        // address indexed _operator, address indexed _tokenHolder
        AuthorizedOperator(AccountId, AccountId),
        // address indexed _operator, address indexed _tokenHolder
        RevokedOperator(AccountId, AccountId),
        // Bytes32 indexed _partition, address indexed _operator, address indexed _tokenHolder
        AuthorizedOperatorByPartition(Bytes32, AccountId, AccountId),
        // Bytes32 indexed _partition, address indexed _operator, address indexed _tokenHolder
        RevokedOperatorByPartition(Bytes32, AccountId, AccountId),

        // Issuance / Redemption Events
        // address indexed _operator, address indexed _to, uint256 _value, bytes _data
        Issued(AccountId, AccountId, Balance, string),
        // address indexed _operator, address indexed _from, uint256 _value, bytes _data
        Redeemed(AccountId, AccountId, Balance, string),
        // Bytes32 indexed _partition, address indexed _operator, address indexed _to, uint256 _value, bytes _data, bytes _operatorData
        IssuedByPartition(Bytes32, AccountId, AccountId, Balance, string, string),
        // Bytes32 indexed _partition, address indexed _operator, address indexed _from, uint256 _value, bytes _operatorData
        RedeemedByPartition(Bytes32, AccountId, AccountId, Balance, string),
    }
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    /// internal transfer function
    fn transfer_impl(from: T::AccountId, to: T::AccountId, value: T::TokenBalance) -> Result {
        ensure!(
            <Balances<T>>::exists(from.clone()),
            "Account does not own this token"
        );
        let balance_from = Self::balance_of(from.clone());
        ensure!(balance_from >= value, "Not enough balance.");

        // update the balances
        let new_balance_from = balance_from
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of(to.clone());
        let new_balance_to = balance_to
            .checked_add(&value)
            .ok_or("overflow in adding balance")?;

        <Balances<T>>::insert(from.clone(), new_balance_from);
        <Balances<T>>::insert(to.clone(), new_balance_to);

        Self::deposit_event(RawEvent::Transfer(from, to, value));
        Ok(())
    }

    // ---ERC777 begin---
    fn _is_operator_for(operator: T::AccountId, token_holder: T::AccountId) -> bool {
        operator == token_holder
            || Self::authorized_operator((operator, token_holder))
            || (Self::is_controllable() && Self::is_controller(operator))
    }

    fn _transfer_with_data(
        partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
        prevent_locking: bool,
    ) -> Result {
        ensure!(
            <Balances<T>>::exists(from.clone()),
            "Account does not own this token"
        );
        ensure!(
            Self::_is_multiple(value.clone()),
            "A9: Transfer Blocked - Token granularity"
        );
        // ensure!(to.clone() != T::AccountId::sa(0), "A6: Transfer Blocked - Receiver not eligible");
        ensure!(
            Self::balance_of(from.clone()) >= value.clone(),
            "A4: Transfer Blocked - Sender balance insufficient"
        );

        // TODO: call_sender
        // _call_sender(partition.clone(), operator.clone(), from.clone(), to.clone(), value.clone(), data.clone(), operator_data.clone());

        // update the balances
        let balance_from = Self::balance_of(from.clone());
        let new_balance_from = balance_from
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        let balance_to = Self::balance_of(to.clone());
        let new_balance_to = balance_to
            .checked_add(&value)
            .ok_or("overflow in adding balance")?;

        <Balances<T>>::insert(from.clone(), new_balance_from);
        <Balances<T>>::insert(to.clone(), new_balance_to);

        // TODO: call_recipient

        Self::deposit_event(RawEvent::TransferWithData(
            operator,
            from,
            to,
            value,
            data,
            operator_data,
        ));
        Ok(())
    }

    fn _redeem(
        partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
    ) -> Result {
        ensure!(
            Self::_is_multiple(value.clone()),
            "A9: Transfer Blocked - Token granularity"
        );
        //  "A5: Transfer Blocked - Sender not eligible" ?

        let balance_from = Self::balance_of(from.clone());
        ensure!(
            balance_from >= value.clone(),
            "A4: Transfer Blocked - Sender balance insufficient"
        );

        // TODO: callsender

        let new_balance_from = balance_from
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        let total_supply = Self::total_supply();
        let new_total_supply = total_supply
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        <Balances<T>>::insert(from.clone(), new_balance_from);
        <TotalSupply<T>>::put(new_total_supply);

        Self::deposit_event(RawEvent::Redeemed(operator, from, value, data));
        Ok(())
    }

    fn _is_multiple(value: T::TokenBalance) -> bool {
        value / Self::granularity() * Self::granularity() == value
    }
    // ---ERC777 end---

    // ---ERC1410 begin---
    fn _transfer_by_partition(
        partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
    ) -> Result {
        ensure!(
            Self::balance_of_by_partition((from.clone(), partition.clone())) >= value.clone(),
            "A4: Transfer Blocked - Sender balance insufficient"
        ); // ensure enough funds
        let mut to_partition = partition.clone();
        if operator_data.len() != 0 && data.len() != 0 {
            to_partition = Self::_get_destination_partition(partition.clone(), data.clone());
        }

        Self::_remove_token_from_partition(from.clone(), partition.clone(), value.clone());
        Self::_transfer_with_data(
            partition.clone(),
            operator.clone(),
            from.clone(),
            to.clone(),
            value.clone(),
            data.clone(),
            operator_data.clone(),
            true,
        );
        Self::_add_token_to_partition(to.clone(), to_partition.clone(), value.clone());

        Self::deposit_event(RawEvent::TransferByPartition(
            partition.clone(),
            operator.clone(),
            from.clone(),
            to.clone(),
            value.clone(),
            data.clone(),
            operator_data.clone(),
        ));

        if to_partition.clone() != partition.clone() {
            Self::deposit_event(RawEvent::ChangedPartition(partition, to_partition, value));
        }
        Ok(())
    }

    /// Indicate whether the operator address is an operator of the tokenHolder
    /// address for the given partition.
    fn _is_operator_for_partition(
        partition: Bytes32,
        operator: T::AccountId,
        token_holder: T::AccountId,
    ) -> bool {
        (Self::_is_operator_for(operator.clone(), token_holder.clone()))
            || Self::authorize_operator_by_partition((
                token_holder.clone(),
                partition.clone(),
                operator.clone(),
            ))
            || (Self::is_controllable() && Self::is_controller_by_partition((partition, operator)))
    }

    /// Remove a token from a specific partition.
    fn _remove_token_from_partition(
        from: T::AccountId,
        partition: Bytes32,
        value: T::TokenBalance,
    ) -> Result {
        let balance_of_by_partition =
            Self::balance_of_by_partition((from.clone(), partition.clone()));
        let new_balance_of_by_partition = balance_of_by_partition
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;
        let total_supply_by_partition = Self::total_supply_by_partition(partition.clone());
        let new_total_supply_by_partition = total_supply_by_partition
            .checked_sub(&value)
            .ok_or("underflow in subtracting balance")?;

        <BalancesPartition<T>>::insert(
            (from.clone(), new_balance_of_by_partition.clone()),
            new_balance_of_by_partition.clone(),
        );
        <ToTalSupplyByPartition<T>>::insert(partition.clone(), new_total_supply_by_partition);

        // If the balance of the TokenHolder's partition is zero, finds and deletes the partition.
        if Self::balance_of_by_partition((from.clone(), partition.clone()))
            == T::TokenBalance::sa(0)
        {
            for i in 0..Self::partitions_of_count() {
                if Self::partitions_of((from.clone(), i)) == partition {
                    <PartitionsOf<T>>::remove((from.clone(), i));
                    <PartitionsOfCount<T>>::put(Self::partitions_of_count() - 1);
                    break;
                }
            }
        }

        // If the total supply is zero, finds and deletes the partition.
        if Self::total_supply_by_partition(partition.clone()) == T::TokenBalance::sa(0) {
            for i in 0..Self::total_partitions_count() {
                if Self::total_partitions(i) == partition.clone() {
                    <TotalPartitions<T>>::remove(i);
                    <TotalPartitionsCount<T>>::put(Self::total_partitions_count() - 1);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Add a token to a specific partition.
    fn _add_token_to_partition(
        to: T::AccountId,
        partition: Bytes32,
        value: T::TokenBalance,
    ) -> Result {
        if value != T::TokenBalance::sa(0) {
            if Self::balance_of_by_partition((to.clone(), partition.clone()))
                == T::TokenBalance::sa(0)
            {
                // push to PartitionsOf
                <PartitionsOf<T>>::insert(Self::partitions_of_count(), partition.clone());
                <PartitionsOfCount<T>>::put(Self::partitions_of_count() + 1);
            }
            <BalancesPartition<T>>::mutate((to.clone(), partition.clone()), |balance| {
                *balance = *balance + value.clone()
            });

            if Self::total_supply_by_partition(partition.clone()) == T::TokenBalance::sa(0) {
                // push to TotalPartitons
                <TotalPartitions<T>>::insert(Self::total_partitions_count(), partition.clone());
                <TotalPartitionsCount<T>>::mutate(|count| {
                    *count += 1;
                });
            }
            <ToTalSupplyByPartition<T>>::mutate(partition.clone(), |total_supply| {
                *total_supply = *total_supply + value
            });
        }

        Ok(())
    }

    // TODO: Retrieve the destination partition from the 'data' field.

    /**
    	* function _getDestinationPartition(Bytes32 fromPartition, bytes memory data) internal pure returns(Bytes32 toPartition) {
     * Bytes32 changePartitionFlag = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
     * Bytes32 flag;
     * assembly {
     * 	flag := mload(add(data, 32))
     * }
     * if(flag == changePartitionFlag) {
     *   assembly {
     *     toPartition := mload(add(data, 64))
     *   }
     * } else {
     *  toPartition = fromPartition;
     * }
    	* }  
     */

    /// Retrieve the destination partition from the 'data' field.
    fn _get_destination_partition(from_partition: Bytes32, data: Vec<u8>) -> Result {
        Ok(())
    }

    fn _get_default_partitions(token_holder: T::AccountId) -> Vec<Bytes32> {
        if Self::default_partitions_of(token_holder.clone()).len() != 0 {
            return Self::default_partitions_of(token_holder.clone());
        } else {
            Self::token_default_partitions()
        }
    }
    // ---ERC1410 end---

    // ---ERC1400 begin---
    fn _redeem_by_partition(
        from_partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
    ) -> Result {
        ensure!(
            Self::balance_of_by_partition((from.clone(), from_partition.clone())) >= value.clone(),
            "A4: Transfer Blocked - Sender balance insufficient"
        );

        Self::_remove_token_from_partition(from.clone(), from_partition.clone(), value.clone());
        Self::_redeem(
            from_partition.clone(),
            operator.clone(),
            from.clone(),
            value.clone(),
            data.clone(),
            // operator_data.clone()
        );

        Self::deposit_event(RawEvent::RedeemedByPartition(
            from_partition,
            operator,
            from,
            value,
            data,
            // operator_data
        ));

        Ok(())
    }

    // TODO: returns (byte, Bytes32, Bytes32) ?
    fn _can_transfer(
        partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
    ) -> Result {
        Ok(())
    }

    fn _issue_by_partition(
        to_partition: Bytes32,
        operator: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
    ) -> Result {
        // Self::_issue(
        //     to_partition.clone(),
        //     operator.clone(),
        //     to.clone(),
        //     value.clone(),
        //     data.clone(),
        //     operator_data.clone(),
        // );
        Self::_add_token_to_partition(to.clone(), to_partition.clone(), value.clone());

        Self::deposit_event(RawEvent::IssuedByPartition(
            to_partition,
            operator,
            to,
            value,
            data,
            operator_data,
        ));
        Ok(())
    }
    // ---ERC1400 end---
}
