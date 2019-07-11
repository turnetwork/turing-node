/// A simple implementation of the ERC1400, reference: https://github.com/ethereum/EIPs/issues/1411
use parity_codec::{Codec, Decode, Encode};
use rstd::prelude::*;
use runtime_primitives::traits::{As, CheckedAdd, CheckedSub, Member, SimpleArithmetic};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, Parameter, StorageMap,
    StorageValue,
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
// H => Hash
pub struct Doc<H> {
    doc_uri: Vec<u8>,
    doc_hash: H,
}

pub type Bytes32 = [u8; 32];

// /**
//  * Reason codes - ERC1066
//  *
//  * To improve the token holder experience, canTransfer MUST return a reason byte code
//  * on success or failure based on the EIP-1066 application-specific status codes specified below.
//  * An implementation can also return arbitrary data as a bytes32 to provide additional
//  * information not captured by the reason code.
//  *
//  * Code	Reason
//  * 0xA0	Transfer Verified - Unrestricted
//  * 0xA1	Transfer Verified - On-Chain approval for restricted token
//  * 0xA2	Transfer Verified - Off-Chain approval for restricted token
//  * 0xA3	Transfer Blocked - Sender lockup period not ended
//  * 0xA4	Transfer Blocked - Sender balance insufficient
//  * 0xA5	Transfer Blocked - Sender not eligible
//  * 0xA6	Transfer Blocked - Receiver not eligible
//  * 0xA7	Transfer Blocked - Identity restriction
//  * 0xA8	Transfer Blocked - Token restriction
//  * 0xA9	Transfer Blocked - Token granularity
//  */


// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as ERC1400 {
        // use config() to set the data in the genesis config
        Owner get(owner) config(): T::AccountId;

        // ---Certificate begin---

        // Address used by off-chain controller service to sign certificate
        CertificateSigners get(certificate_signers): map T::AccountId => bool;

        // A nonce used to ensure a certificate can be used only once
        CheckCount get(check_count): map T::AccountId => u64;

        // ---Certificate end---

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
        // ---ERC777 end---

        // ---ERC1410 begin---
        // List of partitions.
        TotalPartitions get(total_partitions): map u64 => Bytes32;
        TotalPartitionsCount get(total_partitions_count): u64;

        // Mapping from partition to global balance of corresponding partition.
        ToTalSupplyByPartition get(total_supply_by_partition): map Bytes32 => T::TokenBalance;

        // Mapping from tokenHolder to their partitions.
        PartitionsOf get(partitions_of): map (T::AccountId, u64) => Bytes32;
        PartitionsOfCount get(partitions_of_count): map T::AccountId => u64;

        // Mapping from (tokenHolder, partition) to balance of corresponding partition.
        BalancesPartition get(balance_of_by_partition): map (T::AccountId, Bytes32) => T::TokenBalance;

        // Mapping from tokenHolder to their default partitions (for ERC777 and ERC20 compatibility).
        DefaultPartitionsOf get(default_partitions_of): map T::AccountId => Vec<Bytes32>;

        // List of token default partitions (for ERC20 compatibility).
        TokenDefaultPartitions get(token_default_partitions) config(): Vec<Bytes32>;

        // Mapping from (tokenHolder, partition, operator) to 'approved for partition' status. [TOKEN-HOLDER-SPECIFIC]
        AuthorizedOperatorByPartition get(authorized_operator_by_partition): map (T::AccountId, Bytes32, T::AccountId) => bool;

        // Mapping from partition to controllers for the partition. [NOT TOKEN-HOLDER-SPECIFIC]
        ControllersByPartition get(controllers_by_partition): map Bytes32 => Vec<T::AccountId>;

        // Mapping from (partition, operator) to PartitionController status. [NOT TOKEN-HOLDER-SPECIFIC]
        IsControllerByPartition get(is_controller_by_partition): map (Bytes32, T::AccountId) => bool;

        // Mapping from (partition, operator, token_holder)
        IsOperatorForPartition get(is_operator_for_partition): map (Bytes32, T::AccountId, T::AccountId) => Option<bool>;
        // ---ERC1410 end---

        // ---ERC1400 begin---
        Granularity get(granularity): T::TokenBalance;

        Documents get(get_document): map Vec<u8> => Doc<T::Hash>;
        Issuable get(is_issuable): bool = true;

        Operator get(operator): map (T::AccountId, T::AccountId) => bool;
        OperatorForPartition get(operator_for_partition): map (Bytes32, T::AccountId, T::AccountId) => bool;
        // ---ERC1400 end---

        // ---ERC20 compatibility begin---
        // Mapping from (token_holder, spender) to allowed value
        Allowances get(allowance): map (T::AccountId, T::AccountId) => T::TokenBalance;

        // Mapping from (token_holder) to whitelisted status.
        Whitelisted get(whitelisted): map T::AccountId => bool;

        // ---ERC20 compatibility end---
    }

    add_extra_genesis {
        build(|storage: &mut runtime_primitives::StorageOverlay, _: &mut runtime_primitives::ChildrenStorageOverlay, config: &GenesisConfig<T>| {
            with_storage(storage, || {
                <Balances<T>>::insert(config.owner.clone(), config.total_supply.clone());
                <IsController<T>>::insert(config.owner.clone(), true);
                <Controllers<T>>::mutate(|c| c.push(config.owner.clone()));
                for p in config.token_default_partitions.clone() {
                    <ControllersByPartition<T>>::insert(p.clone(), vec![config.owner.clone()]);
                    <IsControllerByPartition<T>>::insert((p, config.owner.clone()), true);
                }
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

        // ---Certificate begin---
        fn is_valid_certificate(origin, data: Vec<u8>) -> Result{
            let sender = ensure_signed(origin)?;
            Self::_is_valid_certificate(sender, data)
        }
        // ---Certificate end---

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
                <IsOperatorFor<T>>::insert((operator, token_holder), is_for);
            }
            Ok(())
        }
        // ---ERC777 end---

        // ---ERC1410 begin---

        /// Transfer tokens from a specific partition.
        fn transfer_by_partition(
            origin,
            partition: Bytes32,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }
            Self::_transfer_by_partition(partition, sender.clone(), sender, to, value, data, "".into())
        }

        /// Transfer tokens from a specific partition through an operator.
        fn operator_transfer_by_partition(
            origin,
            partition: Bytes32,
            from: T::AccountId,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), operator_data.clone()){
                return Err(r);
            }
            ensure!(Self::_is_operator_for_partition(partition.clone(), sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");

            Self::_transfer_by_partition(partition, sender, from, to, value, data, operator_data)
        }

        /// Set default partitions to transfer from.
        /// Function used for ERC777 and ERC20 backwards compatibility.
        fn set_default_partitons(origin, partitions: Vec<Bytes32>) -> Result {
            let sender = ensure_signed(origin)?;
            <DefaultPartitionsOf<T>>::insert(sender, partitions);
            Ok(())
        }

        /// Set 'operator' as an operator for 'msg.sender' for a given partition.
        fn authorize_operator_by_partition(origin, partition: Bytes32, operator: T::AccountId) -> Result {
            let sender = ensure_signed(origin)?;
            <AuthorizedOperatorByPartition<T>>::insert((sender.clone(), partition.clone(), operator.clone()), true);
            Self::deposit_event(RawEvent::AuthorizedOperatorByPartition(partition, operator, sender));
            Ok(())
        }

        /// Remove the right of the operator address to be an operator on a given
        /// partition for 'msg.sender' and to transfer and redeem tokens on its behalf.
        fn revoke_operator_by_partition(origin, partition: Bytes32, operator: T::AccountId) -> Result {
            let sender = ensure_signed(origin)?;
            <AuthorizedOperatorByPartition<T>>::insert((sender.clone(), partition.clone(), operator.clone()), false);
            Self::deposit_event(RawEvent::RevokedOperatorByPartition(partition, operator, sender));
            Ok(())
        }

        fn check_operator_for_partition(partition: Bytes32, operator: T::AccountId, token_holder: T::AccountId) -> Result {
            let result = Self::is_operator_for((operator.clone(), token_holder.clone()));
            if result == None {
                let is_for = Self::_is_operator_for_partition(partition.clone(), operator.clone(), token_holder.clone());
                <IsOperatorForPartition<T>>::insert((partition, operator, token_holder), is_for);
            }
            Ok(())
        }

        fn transfer_with_data(origin, to: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result{
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }
            Self::_transfer_by_default_partitions(sender.clone(), sender, to, value, data, "".into())
        }

        fn transfer_from_with_data(
            origin,
            from: T::AccountId,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }
            ensure!(Self::_is_operator_for(sender.clone(), from.clone()), "A7: Transfer Blocked - Identity restriction");
            Self::_transfer_by_default_partitions(sender, from, to, value, data, operator_data)
        }

        // ---ERC1410 end---

        // ---ERC1400 begin---
        fn set_document(name: Vec<u8>, uri: Vec<u8>, document_hash: T::Hash) -> Result {
            let d = Doc{
                doc_uri: uri.clone(),
                doc_hash: document_hash,
            };
            <Documents<T>>::insert(name.clone(), d);
            Self::deposit_event(RawEvent::Document(name, uri, document_hash));
            Ok(())
        }

        fn controller_transfer(
            origin,
            from: T::AccountId,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::is_controllable(), "Is not controllable any more");
            ensure!(Self::is_controller(sender.clone()), "A7: Transfer Blocked - Identity restriction");

            if let Err(r) = Self::_transfer_by_default_partitions(sender.clone(), from.clone(), to.clone(), value, data.clone(), operator_data.clone()) {
                return Err(r);
            } else {
                Self::deposit_event(RawEvent::ControllerTransfer(sender, from, to, value, data, operator_data));
                Ok(())
            }
        }

        fn controller_redeem(
            origin,
            token_holder: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
             let sender = ensure_signed(origin)?;
            ensure!(Self::is_controllable(), "Is not controllable any more");
            ensure!(Self::is_controller(sender.clone()), "A7: Transfer Blocked - Identity restriction");

            if let Err(r) = Self::_redeem_by_default_partition(sender.clone(), token_holder.clone(), value, data.clone(), operator_data.clone()) {
                return Err(r);
            } else {
                Self::deposit_event(RawEvent::ControllerRedemption(sender, token_holder, value ,data, operator_data));
                Ok(())
            }
        }

        fn issue(origin, token_holder: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");
            ensure!(Self::is_issuable(), "A8: Transfer Blocked - Token restriction");
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }

            // issued partition
            Self::_issue_by_partition(Self::token_default_partitions()[1], sender, token_holder, value, data, "".into())
        }

        /// Issue tokens from a specific partition.
        fn issue_by_partition(origin, partition: Bytes32, token_holder: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");
            ensure!(Self::is_issuable(), "A8: Transfer Blocked - Token restriction");
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }
            Self::_issue_by_partition(partition, sender, token_holder, value, data, "".into())
        }

        /// Redeem tokens of a specific partition.
        fn redeem_by_partition(origin, partition: Bytes32, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }
            Self::_redeem_by_partition(partition, sender.clone(), sender, value, data, "".into())
        }

        /// Redeem tokens of a specific partition.
        fn operator_redeem_by_partition(
            origin,
            partition: Bytes32,
            token_holder: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>,
            operator_data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), operator_data.clone()){
                return Err(r);
            }
            ensure!(Self::is_operator_for_partition((
                partition.clone(),
                sender.clone(),
                token_holder.clone()
                )) == Some(true), "A7: Transfer Blocked - Identity restriction");

            Self::_redeem_by_partition(partition, sender, token_holder, value, data, operator_data)
        }

        fn can_transfer(
            origin,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            
            for i in 0..Self::partitions_of_count(sender.clone()) {
                    if let Err(_) = Self::_can_transfer(Self::partitions_of((sender.clone(), i)), sender.clone(), sender.clone(), to.clone(), value, data.clone(), "".into()) {
                        continue;
                    } else {
                        return Ok(())
                    }
            }

            Err("Can not transfer")
        }

        /// Know the reason on success or failure based on the EIP-1066 application-specific status codes.
        fn can_transfer_by_partition(
            origin,
            partition: Bytes32,
            to: T::AccountId,
            #[compact] value: T::TokenBalance,
            data: Vec<u8>
        ) -> Result {
            let sender = ensure_signed(origin)?;
            if !Self::_check_certificate(data.clone()) {
                return Err("A3");
            } else {
                Self::_can_transfer(partition, sender.clone(), sender, to, value, data, "".into())
            }
        }

        /// Redeem the value of tokens from the address 'sender'.
        fn redeem(origin, #[compact] value: T::TokenBalance, data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()) {
                return Err(r);
            } else {
                Self::_redeem_by_default_partition(sender.clone(), sender.clone(), value, data, "".into())
            }
        }

        fn redeem_from(origin, token_holder: T::AccountId, #[compact] value: T::TokenBalance, data: Vec<u8>, operator_data: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            if let Err(r) = Self::_is_valid_certificate(sender.clone(), data.clone()){
                return Err(r);
            }
            ensure!(Self::_is_operator_for(sender.clone(), token_holder.clone()), "A7: Transfer Blocked - Identity restriction");
            Self::_redeem_by_default_partition(sender, token_holder, value, data, operator_data)
        }

        // optional functions
        /// Definitely renounce the possibility to control tokens on behalf of tokenHolders.
        /// Once set to false, '_isControllable' can never be set to 'true' again.
        fn renounce_control(origin) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");

            <Controllable<T>>::put(false);
            Ok(())
        }

        /// Definitely renounce the possibility to issue new tokens.
        /// Once set to false, '_isIssuable' can never be set to 'true' again.
        fn renounce_issuance(origin) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");

            <Issuable<T>>::put(false);
            Ok(())
        }

        /// Set list of token controllers.
        fn set_controllers(origin, operators: Vec<T::AccountId>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");

            Self::_set_controllers(operators)
        }

        /// Set list of token partition controllers.
        fn set_partition_controllers(origin, partition: Bytes32, operators: Vec<T::AccountId>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");
            
            Self::_set_partition_controllers(partition, operators)
        }

        fn set_certificate_signer(origin, operator: T::AccountId, authorized: bool) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");

            Self::_set_certificate_signer(operator, authorized)
        }

        fn set_token_default_partitions(origin, default_partitions: Vec<Bytes32>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(sender.clone() == Self::owner(), "A7: Transfer Blocked - Identity restriction");

            <TokenDefaultPartitions<T>>::mutate(|p| *p = default_partitions);
            Ok(())
        }

        // ---ERC1400 end---

        // ---ERC20 compatibility begin---
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

        /// Transfers token from the sender to the `to` address.
        fn transfer(origin, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::whitelisted(to.clone()), "A3: Transfer Blocked - Sender lockup period not ended");
            Self::_transfer_by_default_partitions(sender.clone(), sender, to, value, "".into(), "".into())
        }

        /// Transfer tokens from one address to another by allowance
        fn transfer_from(origin, from: T::AccountId, to: T::AccountId, #[compact] value: T::TokenBalance) -> Result {
            // Need to be authorized first
            let caller = ensure_signed(origin)?;
            ensure!(Self::whitelisted(to.clone()), "A3: Transfer Blocked - Sender lockup period not ended");
            let allowance = Self::allowance((from.clone(), caller.clone()));
            ensure!(Self::_is_operator_for(caller.clone() ,from.clone()),
                "A7: Transfer Blocked - Identity restriction");
            ensure!((<Allowances<T>>::exists((from.clone(), caller.clone())) && value <= allowance),
                "A4: Transfer Blocked - Sender balance insufficient");

            let new_allowance = allowance.checked_sub(&value).ok_or("underflow in subtracting allowance.")?;
            <Allowances<T>>::insert((from.clone(), caller.clone()), new_allowance);
            Self::deposit_event(RawEvent::Approval(from.clone(), caller.clone(), value));
            
            Self::_transfer_by_default_partitions(caller, from, to, value, "".into(), "".into())
        }

        fn set_whitelisted(token_holder: T::AccountId, authorized: bool) -> Result {
            Self::_set_whitelisted(token_holder, authorized)
        }
        // ---ERC20 compatibility end---
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as self::Trait>::TokenBalance,
        Str = Vec<u8>,
        Hash = <T as system::Trait>::Hash,
        Bytes32 = Bytes32,
    {
        // ---Certificate begin---
        // sender
        Checked(AccountId),

        // ---Certificate end---
        
        // ---ERC777 begin---
        // operator, from, to, value, data, operator_data
        TransferWithData(AccountId, AccountId, AccountId, Balance, Vec<u8>, Vec<u8>),

        // address indexed _operator, address indexed _to, uint256 _value, bytes _data
        Issued(AccountId, AccountId, Balance, Str, Str),

        // address indexed _operator, address indexed _from, uint256 _value, bytes _data
        Redeemed(AccountId, AccountId, Balance, Str),

        // Operator Events
        // address indexed _operator, address indexed _tokenHolder
        AuthorizedOperator(AccountId, AccountId),

        // address indexed _operator, address indexed _tokenHolder
        RevokedOperator(AccountId, AccountId),

        // ---ERC777 end---

        // ---ERC1410 begin---
        // fromPartition, operator, from, to, value, data, operatorData
        TransferByPartition(Bytes32, AccountId, AccountId, AccountId, Balance, Str, Str),

        // fromPartition, toPartition, value
        ChangedPartition(Bytes32, Bytes32, Balance),

        // Bytes32 indexed _partition, address indexed _operator, address indexed _tokenHolder
        AuthorizedOperatorByPartition(Bytes32, AccountId, AccountId),
        
        // Bytes32 indexed _partition, address indexed _operator, address indexed _tokenHolder
        RevokedOperatorByPartition(Bytes32, AccountId, AccountId),
        // ---ERC1410 end---

        // ---ERC1400 begin---
        // name, uri, document_hash
        Document(Str, Str, Hash),

        // Bytes32 indexed _partition, address indexed _operator, address indexed _to, uint256 _value, bytes _data, bytes _operatorData
        IssuedByPartition(Bytes32, AccountId, AccountId, Balance, Str, Str),

        // Bytes32 indexed _partition, address indexed _operator, address indexed _from, uint256 _value, bytes _operatorData
        RedeemedByPartition(Bytes32, AccountId, AccountId, Balance, Str, Str),

        // controller, from, to, value, data, operatorData
        ControllerTransfer(AccountId, AccountId, AccountId, Balance, Str, Str),
        // controller, token_holder, value, data, operatorData
        ControllerRedemption(AccountId, AccountId, Balance, Str, Str),

        // ---ERC1400 end---

        // ---ERC20 compatibility begin---
        Approval(AccountId, AccountId, Balance),
        // ---ERC20 compatibility end---
    }
);

// utility and private functions
// if marked public, accessible by other modules
impl<T: Trait> Module<T> {
    // ---Certificate begin---
    fn _is_valid_certificate(sender: T::AccountId, data: Vec<u8>) -> Result {
        ensure!(
            Self::certificate_signers(sender.clone()) == true || Self::_check_certificate(data),
            "A3: Transfer Blocked - Sender lockup period not ended"
        );
        <CheckCount<T>>::mutate(sender.clone(), |count| {
            *count += 1;
        });

        Self::deposit_event(RawEvent::Checked(sender));

        Ok(())
    }

    fn _set_certificate_signer(operator: T::AccountId, authorized: bool) -> Result{
        <CertificateSigners<T>>::insert(operator, authorized);
        Ok(())
    }

    fn _check_certificate(
        data: Vec<u8>, // _value: T::TokenBalance,
                       // _function_id: u64
    ) -> bool {
        if data.len() > 0 {
            return true;
        } else {
            false
        }
    }
    // ---Certificate end---

    // ---ERC777 begin---
    fn _issue(
        operator: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>
    ) -> Result {
        ensure!(Self::_is_multiple(value), "A9: Transfer Blocked - Token granularity");
        
        <TotalSupply<T>>::mutate(|total_supply| {*total_supply = *total_supply + value;});
        <Balances<T>>::mutate(to.clone(), |balance| {*balance += value;});

        Self::deposit_event(RawEvent::Issued(operator, to, value, data, operator_data));
        Ok(())
    }

    fn _is_operator_for(operator: T::AccountId, token_holder: T::AccountId) -> bool {
        operator == token_holder
            || Self::authorized_operator((operator.clone(), token_holder))
            || (Self::is_controllable() && Self::is_controller(operator))
    }

    fn _transfer_with_data(
        _partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
        _prevent_locking: bool,
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
        _partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        _operator_data: Vec<u8>,
    ) -> Result {
        ensure!(
            Self::_is_multiple(value.clone()),
            "A9: Transfer Blocked - Token granularity"
        );

        let balance_from = Self::balance_of(from.clone());
        ensure!(
            balance_from >= value.clone(),
            "A4: Transfer Blocked - Sender balance insufficient"
        );

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

    fn _set_controllers(operators: Vec<T::AccountId>) -> Result {
        for i in Self::controllers().iter() {
            <IsController<T>>::mutate(i, |is| *is = false);
        }
        for j in 0..operators.len() {
            <IsController<T>>::mutate(operators[j].clone(), |is| *is = true);
        }
        <Controllers<T>>::mutate(|controllers| *controllers = operators);

        Ok(())
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

        if let Err(r) = Self::_remove_token_from_partition(from.clone(), partition.clone(), value.clone()) {
            return Err(r);
        }
        if let Err(r) = Self::_transfer_with_data(
            partition.clone(),
            operator.clone(),
            from.clone(),
            to.clone(),
            value.clone(),
            data.clone(),
            operator_data.clone(),
            true,
        ) {
            return Err(r);
        }
        if let Err(r) = Self::_add_token_to_partition(to.clone(), to_partition.clone(), value.clone()) {
            return Err(r);
        }

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
            || Self::authorized_operator_by_partition((
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
            (from.clone(), partition.clone()),
            new_balance_of_by_partition.clone(),
        );
        <ToTalSupplyByPartition<T>>::insert(partition.clone(), new_total_supply_by_partition);

        // If the balance of the TokenHolder's partition is zero, finds and deletes the partition.
        if Self::balance_of_by_partition((from.clone(), partition.clone()))
            == T::TokenBalance::sa(0)
        {
            for i in 0..Self::partitions_of_count(from.clone()) {
                if Self::partitions_of((from.clone(), i)) == partition {
                    <PartitionsOf<T>>::remove((from.clone(), i));
                    <PartitionsOfCount<T>>::insert(
                        from.clone(),
                        Self::partitions_of_count(from.clone()) as u64 - 1,
                    );
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
                let count = Self::partitions_of_count(to.clone());
                <PartitionsOf<T>>::insert((to.clone(), count), partition.clone());
                <PartitionsOfCount<T>>::insert(to.clone(), count + 1);
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

    /// Retrieve the destination partition from the 'data' field.
    fn _get_destination_partition(from_partition: Bytes32, data: Vec<u8>) -> Bytes32 {
        let change_partition_flag: Bytes32 = [255;32];
        let mut flag: Bytes32 = Default::default();
        let mut to_partition: Bytes32 = Default::default();
        flag.copy_from_slice(&data[0..32]);
        if flag == change_partition_flag {
            to_partition.copy_from_slice(&data[32..64]);
        } else {
            to_partition = from_partition;
        }
        to_partition
    }

    fn _get_default_partitions(token_holder: T::AccountId) -> Vec<Bytes32> {
        if Self::default_partitions_of(token_holder.clone()).len() != 0 {
            return Self::default_partitions_of(token_holder.clone());
        } else {
            Self::token_default_partitions()
        }
    }

    fn _set_partition_controllers(partition: Bytes32, operators: Vec<T::AccountId>) -> Result {
        for i in Self::controllers_by_partition(partition.clone()).iter() {
            <IsControllerByPartition<T>>::insert((partition, i.clone()), false);
        }
        for j in operators.iter() {
            <IsControllerByPartition<T>>::insert((partition.clone(), j.clone()), true);
        }

        <ControllersByPartition<T>>::mutate(partition, |controllers| *controllers = operators);
        Ok(())
    }

    // TODO: check the logic
    fn _transfer_by_default_partitions(
        operator: T::AccountId,
        from: T::AccountId,
        to: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>
    ) -> Result {
        let partitions = Self::_get_default_partitions(from.clone());
        ensure!(partitions.len() != 0, "A8: Transfer Blocked - Token restriction");

        let mut remaining_value = value;

        for i in 0..partitions.len() {
            let local_balance = Self::balance_of_by_partition((from.clone(), partitions[i]));
            if remaining_value <= local_balance {
                if let Err(r) = Self::_transfer_by_partition(
                    partitions[i],
                    operator.clone(),
                    from.clone(),
                    to.clone(),
                    remaining_value,
                    data.clone(),
                    operator_data.clone(),
                ) {
                    return Err(r);
                } else {
                    remaining_value = T::TokenBalance::sa(0);
                    break;
                }
            } else {
                if let Err(r) = Self::_redeem_by_partition(
                    partitions[i],
                    operator.clone(),
                    from.clone(),
                    local_balance,
                    data.clone(),
                    operator_data.clone(),
                ) {
                    return Err(r);
                } else {
                    remaining_value -= local_balance;
                }
            }
        }

        ensure!(
            remaining_value == T::TokenBalance::sa(0),
            "A8: Transfer Blocked - Token restriction"
        );
        Ok(())
    }
    // ---ERC1410 end---
  
    // ---ERC1400 begin---
    /// Redeem tokens from a default partitions.
    // TODO: check the logic
    fn _redeem_by_default_partition(
        operator: T::AccountId,
        from: T::AccountId,
        value: T::TokenBalance,
        data: Vec<u8>,
        operator_data: Vec<u8>,
    ) -> Result {
        let partitions = Self::_get_default_partitions(from.clone());
        ensure!(
            partitions.len() != 0,
            "A8: Transfer Blocked - Token restriction"
        );

        let mut remaining_value = value;

        for i in 0..partitions.len() {
            let local_balance = Self::balance_of_by_partition((from.clone(), partitions[i]));
            if remaining_value <= local_balance {
                let r = Self::_redeem_by_partition(
                    partitions[i],
                    operator.clone(),
                    from.clone(),
                    remaining_value,
                    data.clone(),
                    operator_data.clone(),
                );
                if r.is_ok() {
                    remaining_value = T::TokenBalance::sa(0);
                    break;
                } else {
                    return r;
                }
            } else {
                let r = Self::_redeem_by_partition(
                    partitions[i],
                    operator.clone(),
                    from.clone(),
                    local_balance,
                    data.clone(),
                    operator_data.clone(),
                );
                if r.is_ok() {
                    remaining_value -= local_balance;
                } else {
                    return r;
                }
            }
        }

        ensure!(
            remaining_value == T::TokenBalance::sa(0),
            "A8: Transfer Blocked - Token restriction"
        );
        Ok(())
    }

    /// Redeem tokens of a specific partition.
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

        if let Err(r) =
            Self::_remove_token_from_partition(from.clone(), from_partition.clone(), value.clone())
        {
            return Err(r);
        }
        if let Err(r) = Self::_redeem(
            from_partition.clone(),
            operator.clone(),
            from.clone(),
            value.clone(),
            data.clone(),
            operator_data.clone(),
        ) {
            return Err(r);
        }

        Self::deposit_event(RawEvent::RedeemedByPartition(
            from_partition,
            operator,
            from,
            value,
            data,
            operator_data,
        ));

        Ok(())
    }

    fn _can_transfer(
        partition: Bytes32,
        operator: T::AccountId,
        from: T::AccountId,
        _to: T::AccountId,
        value: T::TokenBalance,
        _data: Vec<u8>,
        _operator_data: Vec<u8>,
    ) -> Result {
        if !Self::_is_operator_for_partition(partition.clone(), operator.clone(), from.clone()) {
            return Err("A7");
        }

        if Self::balance_of(from.clone()) < value.clone() || Self::balance_of_by_partition((from.clone(), partition.clone())) < value {
            return Err("A4");
        }

        if !Self::_is_multiple(value) {
            return Err("A9");
        }

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
        if let Err(r) = Self::_issue(
            operator.clone(),
            to.clone(),
            value.clone(),
            data.clone(),
            operator_data.clone(),
        ) {
            return Err(r);
        }
        if let Err(r) = Self::_add_token_to_partition(to.clone(), to_partition.clone(), value.clone()) {
            return Err(r);
        }

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

    // ---ERC20 compatibility begin---
    fn _set_whitelisted(token_holder: T::AccountId, authorized: bool) -> Result{
        <Whitelisted<T>>::mutate(token_holder, |w| *w = authorized);
        Ok(())
    }
    // ---ERC20 compatibility end---
}
