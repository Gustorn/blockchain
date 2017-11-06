use hex::{FromHex, ToHex};
use ring::{digest, signature};
use serde_json;
use untrusted;
use uuid::Uuid;

use network;

pub const MINER_REWARD: u64 = 1;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Transfer {
    id: Uuid,
    amount: u64,
    sender: String,
    recipient: String,
    signature: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Reward {
    id: Uuid,
    recipient: String,
    amount: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Transaction {
    Transfer(Transfer),
    Reward(Reward),
}

#[derive(Debug, Serialize)]
struct SignedTransfer<'a> {
    id: Uuid,
    sender: &'a str,
    recipient: &'a str,
    amount: u64,
}

impl From<network::Transfer> for Transfer {
    fn from(transfer: network::Transfer) -> Self {
        let id = Uuid::parse_str(&transfer.id).expect("Incorrect transfer UUID");
        Self {
            id,
            amount: transfer.amount,
            sender: transfer.sender,
            recipient: transfer.recipient,
            signature: transfer.signature,
        }
    }
}

impl Transaction {
    pub fn transfer<S: AsRef<str>, R: AsRef<str>, P: AsRef<[u8]>>(
        sender: S,
        recipient: R,
        amount: u64,
        private_key: P,
    ) -> Self {
        let mut transfer = Transfer {
            id: Uuid::new_v4(),
            sender: String::from(sender.as_ref()),
            recipient: String::from(recipient.as_ref()),
            amount,
            signature: String::new(),
        };
        let signature = {
            let message = Transaction::transfer_hash(&transfer);
            let key_pair = signature::Ed25519KeyPair::from_pkcs8(
                untrusted::Input::from(private_key.as_ref()),
            ).expect("Cannot create private/public key pair");
            key_pair.sign(message.as_ref()).to_hex()
        };
        transfer.signature = signature;
        Transaction::Transfer(transfer)
    }

    pub fn reward(recipient: String) -> Self {
        Transaction::Reward(Reward {
            id: Uuid::new_v4(),
            recipient,
            amount: MINER_REWARD,
        })
    }

    pub fn is_valid(&self) -> bool {
        match self {
            &Transaction::Transfer(ref transfer) => Transaction::is_valid_transfer(transfer),
            &Transaction::Reward(ref reward) => reward.amount == MINER_REWARD,
        }
    }

    fn is_valid_transfer(transfer: &Transfer) -> bool {
        let public_key_bytes = Vec::from_hex(&transfer.sender).unwrap();
        let signature_bytes = Vec::from_hex(&transfer.signature).unwrap();

        let public_key = untrusted::Input::from(public_key_bytes.as_ref());
        let signature = untrusted::Input::from(signature_bytes.as_ref());
        let message = Transaction::transfer_hash(transfer);
        signature::verify(
            &signature::ED25519,
            public_key,
            untrusted::Input::from(message.as_ref()),
            signature,
        ).is_ok()
    }

    fn transfer_hash(transfer: &Transfer) -> Vec<u8> {
        let signed_transfer = SignedTransfer {
            id: transfer.id,
            sender: &transfer.sender,
            recipient: &transfer.recipient,
            amount: transfer.amount
        };
        serde_json::to_string(&signed_transfer)
            .map(|message| digest::digest(&digest::SHA512, message.as_ref()))
            .expect("Transactions must be able to generate a hash")
            .as_ref()
            .into()
    }
}
