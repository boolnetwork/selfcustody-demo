use bitcoin::consensus::encode;
use bitcoin::key::Keypair;
use bitcoin::secp256k1::schnorr::Signature;
use bitcoin::secp256k1::XOnlyPublicKey;
use bitcoin::secp256k1::{Message, Secp256k1};
use bitcoin::sighash::TapSighashType;
use bitcoin::taproot::TaprootBuilder;
use bitcoin::TapSighash;
use bitcoin::{Address, Network, ScriptBuf, Transaction, Witness};

pub fn mulsig_address(multisign: Option<String>, private_key_u8: &[u8]) -> String {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_seckey_slice(&secp, private_key_u8).unwrap();

    multisign.unwrap_or(hex::encode(keypair.public_key().serialize()))
}

pub fn build_p2tr_tweaked(
    combined_xonly: XOnlyPublicKey,
    mulsig_script: ScriptBuf,
    escape_script: ScriptBuf,
    network: Network,
) -> Address {
    let secp = Secp256k1::new();
    let taproot_spend_info = TaprootBuilder::new()
        .add_leaf(1, escape_script)
        .expect("adding leaf should work")
        .add_leaf(1, mulsig_script)
        .expect("adding leaf should work")
        .finalize(&secp, combined_xonly)
        .expect("finalizing taproot builder should work");

    Address::p2tr_tweaked(taproot_spend_info.output_key(), network)
}

pub fn calculate_fee(virtual_size: usize, rate: f64, multiplier: f64) -> u64 {
    let kilo_bytes = virtual_size as f64 / 1000_f64;
    let rate = bitcoin::Amount::from_btc(rate).unwrap().to_sat() as f64;
    ((kilo_bytes * rate) * multiplier).round() as u64
}

pub fn sign_transaction(sighash: Vec<TapSighash>, secret: &[u8]) -> Vec<Signature> {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_seckey_slice(&secp, secret).unwrap();
    sighash
        .into_iter()
        .map(|v| secp.sign_schnorr(&Message::from(v), &keypair))
        .collect()
}

pub fn combine_multi_sign_transaction(
    unsigned_tx: Transaction,
    sig1s: Vec<Signature>,
    sig2s: Vec<Signature>,
) -> String {
    let mut unsigned_tx = unsigned_tx;
    assert_eq!(unsigned_tx.input.len(), sig1s.len());
    assert_eq!(unsigned_tx.input.len(), sig2s.len());
    // get witness
    // insert signatures
    for i in 0..unsigned_tx.input.len() {
        let mut witness = Witness::new();
        witness.push(
            bitcoin::taproot::Signature {
                sig: sig1s[i],
                hash_ty: TapSighashType::Default,
            }
            .to_vec(),
        );
        witness.push(
            bitcoin::taproot::Signature {
                sig: sig2s[i],
                hash_ty: TapSighashType::Default,
            }
            .to_vec(),
        );
        witness.push(unsigned_tx.input[i].witness.second_to_last().unwrap());
        witness.push(unsigned_tx.input[i].witness.last().unwrap());
        unsigned_tx.input[i].witness = witness;
    }

    encode::serialize_hex(&unsigned_tx)
}

pub fn combine_escape_transaction(unsigned_tx: Transaction, sig1s: Vec<Signature>) -> String {
    let mut unsigned_tx = unsigned_tx;
    assert_eq!(unsigned_tx.input.len(), sig1s.len());
    for i in 0..unsigned_tx.input.len() {
        let mut witness = Witness::new();
        witness.push(
            bitcoin::taproot::Signature {
                sig: sig1s[i],
                hash_ty: TapSighashType::Default,
            }
            .to_vec(),
        );
        witness.push(unsigned_tx.input[i].witness.second_to_last().unwrap());
        witness.push(unsigned_tx.input[i].witness.last().unwrap());
        unsigned_tx.input[i].witness = witness;
    }

    encode::serialize_hex(&unsigned_tx)
}

pub fn combine_key_transaction(unsigned_tx: Transaction, sig1s: Vec<Signature>) -> String {
    let mut unsigned_tx = unsigned_tx;
    assert_eq!(unsigned_tx.input.len(), sig1s.len());
    for i in 0..unsigned_tx.input.len() {
        let mut witness = Witness::new();
        witness.push(
            bitcoin::taproot::Signature {
                sig: sig1s[i],
                hash_ty: TapSighashType::Default,
            }
            .to_vec(),
        );
        unsigned_tx.input[i].witness = witness;
    }

    encode::serialize_hex(&unsigned_tx)
}
