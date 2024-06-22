// // SPDX-License-Identifier: MIT
// pragma solidity ^0.8.24;

// contract MultiSigWallet {
//     event Deposit(address indexed sender, uint256 amount, uint256 balance);
//     event SubmitTransaction(
//         address indexed owner,
//         uint256 indexed txIndex,
//         address indexed to,
//         uint256 value,
//         bytes data
//     );
//     event ConfirmTransaction(address indexed owner, uint256 indexed txIndex);
//     event RevokeConfirmation(address indexed owner, uint256 indexed txIndex);
//     event ExecuteTransaction(address indexed owner, uint256 indexed txIndex);

//     address[] public owners;
//     mapping(address => bool) public isOwner;
//     uint256 public numConfirmationsRequired;

//     struct Transaction {
//         address to;
//         uint256 value;
//         bytes data;
//         bool executed;
//         uint256 numConfirmations;
//     }

//     // mapping from tx index => owner => bool
//     mapping(uint256 => mapping(address => bool)) public isConfirmed;

//     Transaction[] public transactions;

//     modifier onlyOwner() {
//         require(isOwner[msg.sender], "not owner");
//         _;
//     }

//     modifier txExists(uint256 _txIndex) {
//         require(_txIndex < transactions.length, "tx does not exist");
//         _;
//     }

//     modifier notExecuted(uint256 _txIndex) {
//         require(!transactions[_txIndex].executed, "tx already executed");
//         _;
//     }

//     modifier notConfirmed(uint256 _txIndex) {
//         require(!isConfirmed[_txIndex][msg.sender], "tx already confirmed");
//         _;
//     }

//     constructor(address[] memory _owners, uint256 _numConfirmationsRequired) {
//         require(_owners.length > 0, "owners required");
//         require(
//             _numConfirmationsRequired > 0
//                 && _numConfirmationsRequired <= _owners.length,
//             "invalid number of required confirmations"
//         );

//         for (uint256 i = 0; i < _owners.length; i++) {
//             address owner = _owners[i];

//             require(owner != address(0), "invalid owner");
//             require(!isOwner[owner], "owner not unique");

//             isOwner[owner] = true;
//             owners.push(owner);
//         }

//         numConfirmationsRequired = _numConfirmationsRequired;
//     }

//     receive() external payable {
//         emit Deposit(msg.sender, msg.value, address(this).balance);
//     }

//     function submitTransaction(address _to, uint256 _value, bytes memory _data)
//         public
//         onlyOwner
//     {
//         uint256 txIndex = transactions.length;

//         transactions.push(
//             Transaction({
//                 to: _to,
//                 value: _value,
//                 data: _data,
//                 executed: false,
//                 numConfirmations: 0
//             })
//         );

//         emit SubmitTransaction(msg.sender, txIndex, _to, _value, _data);
//     }

//     function confirmTransaction(uint256 _txIndex)
//         public
//         onlyOwner
//         txExists(_txIndex)
//         notExecuted(_txIndex)
//         notConfirmed(_txIndex)
//     {
//         Transaction storage transaction = transactions[_txIndex];
//         transaction.numConfirmations += 1;
//         isConfirmed[_txIndex][msg.sender] = true;

//         emit ConfirmTransaction(msg.sender, _txIndex);
//     }

//     function executeTransaction(uint256 _txIndex)
//         public
//         onlyOwner
//         txExists(_txIndex)
//         notExecuted(_txIndex)
//     {
//         Transaction storage transaction = transactions[_txIndex];

//         require(
//             transaction.numConfirmations >= numConfirmationsRequired,
//             "cannot execute tx"
//         );

//         transaction.executed = true;

//         (bool success,) =
//             transaction.to.call{value: transaction.value}(transaction.data);
//         require(success, "tx failed");

//         emit ExecuteTransaction(msg.sender, _txIndex);
//     }

//     function revokeConfirmation(uint256 _txIndex)
//         public
//         onlyOwner
//         txExists(_txIndex)
//         notExecuted(_txIndex)
//     {
//         Transaction storage transaction = transactions[_txIndex];

//         require(isConfirmed[_txIndex][msg.sender], "tx not confirmed");

//         transaction.numConfirmations -= 1;
//         isConfirmed[_txIndex][msg.sender] = false;

//         emit RevokeConfirmation(msg.sender, _txIndex);
//     }

//     function getOwners() public view returns (address[] memory) {
//         return owners;
//     }

//     function getTransactionCount() public view returns (uint256) {
//         return transactions.length;
//     }

//     function getTransaction(uint256 _txIndex)
//         public
//         view
//         returns (
//             address to,
//             uint256 value,
//             bytes memory data,
//             bool executed,
//             uint256 numConfirmations
//         )
//     {
//         Transaction storage transaction = transactions[_txIndex];

//         return (
//             transaction.to,
//             transaction.value,
//             transaction.data,
//             transaction.executed,
//             transaction.numConfirmations
//         );
//     }
// }

// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(feature = "export-abi"), no_main)]
extern crate alloc;

/// Use an efficient WASM allocator.
#[global_allocator]
static ALLOC: mini_alloc::MiniAlloc = mini_alloc::MiniAlloc::INIT;

/// Import items from the SDK. The prelude contains common traits and macros.
use stylus_sdk::{contract, evm, msg, prelude::*, call::{Call, call}, alloy_primitives::{Address, U256}};
use alloy_sol_types::sol;
// 
sol! {
    event Deposit(address indexed sender, uint256 amount, uint256 balance);
    event SubmitTransaction(
        address indexed owner,
        uint256 indexed txIndex,
        address indexed to,
        uint256 value,
        bytes data
    );
    event ConfirmTransaction(address indexed owner, uint256 indexed txIndex);
    event RevokeConfirmation(address indexed owner, uint256 indexed txIndex);
    event ExecuteTransaction(address indexed owner, uint256 indexed txIndex);

    error ZeroOwners();
    error NotOwner();
    error TxDoesNotExist();
    error TxAlreadyExecuted();
    error TxAlreadyConfirmed();
    error TxNotConfirmed();
    error ExecuteFailed();
}

// Define some persistent storage using the Solidity ABI.
// `Counter` will be the entrypoint.
sol_storage! {
    #[entrypoint]
    pub struct Counter {
        address[] owners;
        mapping(address => bool) is_owner;
        uint256 num_confirmations_required;
        uint256 number;
        TxStruct[] transactions;
        // mapping from tx index => owner => bool
        mapping(uint256 => mapping(address => bool)) is_confirmed;
    }

    pub struct TxStruct {
        address to;
        uint256 value;
        bytes data;
        bool executed;
        uint256 num_confirmations;
    }
}

// Error types for the TimeLock contract
#[derive(SolidityError)]
pub enum MultiSigError {
    ZeroOwners(ZeroOwners),
    NotOwner(NotOwner),
    TxDoesNotExist(TxDoesNotExist),
    TxAlreadyExecuted(TxAlreadyExecuted),
    TxAlreadyConfirmed(TxAlreadyConfirmed),
    TxNotConfirmed(TxNotConfirmed),
    ExecuteFailed(ExecuteFailed),
}

/// Declare that `Counter` is a contract with the following external methods.
#[external]
impl Counter {
    pub fn deposit(&mut self) {
        let sender = msg::sender();
        let amount = msg::value();
        evm::log(
            Deposit{
                sender: sender, 
                amount: amount, 
                balance: contract::balance()
            });
    }

    pub fn submit_transaction(&mut self, to: Address, value: U256, data: Vec<u8>) {
        let tx_index = U256::from(self.transactions.len());
        
        let mut new_tx = self.transactions.grow();
        new_tx.to.set(to);
        new_tx.value.set(value);
        new_tx.data.set_bytes(data.clone());
        new_tx.executed.set(false);
        new_tx.num_confirmations.set(U256::from(0));

        evm::log(SubmitTransaction {
            owner: msg::sender(),
            txIndex: tx_index,
            to: to,
            value: value,
            data: data.clone(),
        });
    }


    pub fn initialize(&mut self, owners: Vec<Address>, num_confirmations_required: U256) -> Result<(), MultiSigError> {
        if owners.len() == 0 {
            return Err(MultiSigError::ZeroOwners(ZeroOwners{}));
        }

        if num_confirmations_required == U256::from(0) || num_confirmations_required > U256::from(owners.len()) {
            panic!("invalid number of required confirmations");
        }

        for owner in owners.iter() {
            if *owner == Address::zero() {
                panic!("invalid owner");
            }

            if self.is_owner.get(*owner) {
                panic!("owner not unique");
            }

            self.is_owner.setter(*owner).set(true);
            self.owners.push(*owner);
        }

        self.num_confirmations_required.set(num_confirmations_required);
        Ok(())
    }

    pub fn execute_transaction(&mut self, tx_index: U256) -> Result<(), MultiSigError>{
        let tx_index = tx_index.to::<usize>();
        if tx_index >= self.transactions.len() {
            return Err(MultiSigError::TxDoesNotExist(TxDoesNotExist{}));
        }

        if let Some(mut entry) = self.transactions.get_mut(tx_index) {
            if entry.executed.get() {
                return Err(MultiSigError::TxAlreadyExecuted(TxAlreadyExecuted{}));
            }

            if entry.num_confirmations.get() < self.num_confirmations_required.get() {
                return Err(MultiSigError::TxAlreadyConfirmed(TxAlreadyConfirmed{}));
            }
            
            entry.executed.set(true);
            // let executed_setter = entry.executed.setter();
            match call(Call::new().value(entry.value.get()), entry.to.get(), &entry.data.get_bytes()) {
                Ok(_) => {
                    evm::log(ExecuteTransaction {
                        owner: msg::sender(),
                        txIndex: U256::from(tx_index),
                    });
                    Ok(())
                },
                Err(_) => {
                    return Err(MultiSigError::ExecuteFailed(ExecuteFailed{}));
                }
            }
            
        } else {
            return Err(MultiSigError::TxDoesNotExist(TxDoesNotExist{}));
        }
    }

    pub fn revoke_confirmation(&mut self, tx_index: U256) -> Result<(), MultiSigError> {
        // let tx_index = tx_index.to;
        if tx_index >= U256::from(self.transactions.len()) {
            return Err(MultiSigError::TxDoesNotExist(TxDoesNotExist{}));
        }

        if let Some(mut entry) = self.transactions.get_mut(tx_index) {
            if !self.is_confirmed.get(tx_index).get(msg::sender()) {
                return Err(MultiSigError::TxNotConfirmed(TxNotConfirmed{}));
            }

            let num_confirmations = entry.num_confirmations.get();
            entry.num_confirmations.set(num_confirmations - U256::from(1));
            let mut tx_confirmed_info = self.is_confirmed.setter(tx_index);
            let mut confirmed_by_address = tx_confirmed_info.setter(msg::sender());
            confirmed_by_address.set(false);

            evm::log(RevokeConfirmation {
                owner: msg::sender(),
                txIndex: U256::from(tx_index),
            });
            Ok(())
        } else {
            return Err(MultiSigError::TxDoesNotExist(TxDoesNotExist{}));
        }
    }

    pub fn is_owner(&self, owner: Address) -> bool {
        self.is_owner.get(owner)
    }

    pub fn get_transaction_count(&self) -> U256 {
        U256::from(self.transactions.len())
    }

    pub fn get_transaction(&self, tx_index: U256) -> Result<(Address, U256, Vec<u8>, bool, U256), MultiSigError> {
        let tx_index = tx_index.to::<usize>();
        if tx_index >= self.transactions.len() {
            return Err(MultiSigError::TxDoesNotExist(TxDoesNotExist{}));
        }

        if let Some(entry) = self.transactions.get(tx_index) {
            Ok((
                entry.to.get(),
                entry.value.get(),
                entry.data.get_bytes(),
                entry.executed.get(),
                entry.num_confirmations.get(),
            ))
        } else {
            return Err(MultiSigError::TxDoesNotExist(TxDoesNotExist{}));
        }
    }
}


