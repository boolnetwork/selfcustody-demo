use bitcoin::absolute::LockTime;

use bitcoin::secp256k1::schnorr::Signature;
use bitcoin::secp256k1::XOnlyPublicKey;
use std::str::FromStr;

use crate::aux::calculate_fee;
use crate::LocalUtxo;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TaprootBuilder};
use bitcoin::TapLeafHash;
use bitcoin::TapSighash;
use bitcoin::{
    script, transaction, Address, Amount, Network, OutPoint, ScriptBuf, Sequence, TapNodeHash,
    Transaction, TxIn, TxOut, Txid, Witness,
};

pub enum SpendType {
    MULSIG,
    ESCAPE(u32),
    KEY,
}

pub(crate) fn build_tx(
    spent_type: SpendType,
    combined_xonly: XOnlyPublicKey,
    mulsig_script: ScriptBuf,
    escape_script: ScriptBuf,
    network: Network,
    utxos: Vec<LocalUtxo>,
    fee_rate: f64,
    receiver: Address,
    amount: u64,
) -> (Transaction, Vec<TapSighash>, Option<TapNodeHash>) {
    let secp = Secp256k1::new();

    let taproot_spend_info = TaprootBuilder::new()
        .add_leaf(1, escape_script.clone())
        .expect("adding leaf should work")
        .add_leaf(1, mulsig_script.clone())
        .expect("adding leaf should work")
        .finalize(&secp, combined_xonly)
        .expect("finalizing taproot builder should work");

    let mut lock_time = LockTime::ZERO;
    let script = match spent_type {
        SpendType::MULSIG => mulsig_script.clone(),
        SpendType::ESCAPE(t) => {
            lock_time = LockTime::from_consensus(t);
            escape_script.clone()
        }
        _ => escape_script.clone(),
    };
    let control_block = taproot_spend_info
        .control_block(&(script, LeafVersion::TapScript))
        .expect("should compute control block");

    let addr_self = Address::p2tr_tweaked(taproot_spend_info.output_key(), network);

    let total_amount = utxos.iter().map(|v| v.amount).sum();

    let spend_to_receiver = TxOut {
        value: Amount::from_sat(amount),
        script_pubkey: receiver.script_pubkey(),
    };
    let spend_to_owner = TxOut {
        value: Amount::from_sat(total_amount),
        script_pubkey: addr_self.script_pubkey(),
    };
    let mut unsigned_tx = Transaction {
        version: transaction::Version::ONE,
        lock_time,
        input: utxos
            .iter()
            .map(|input| TxIn {
                previous_output: OutPoint {
                    txid: Txid::from_str(&input.txid).unwrap(),
                    vout: input.vout,
                },
                script_sig: script::Builder::new().into_script(),
                witness: Witness::new(),
                sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            })
            .collect(),
        output: vec![spend_to_receiver, spend_to_owner],
    };

    // mock witness data structure
    let mock_signature = bitcoin::taproot::Signature {
        sig: Signature::from_slice(&[0; 64]).unwrap(),
        hash_ty: TapSighashType::Default,
    }
    .to_vec();
    let mut witness = Witness::new();
    witness.push(mock_signature.clone());
    match spent_type {
        SpendType::MULSIG => {
            witness.push(mock_signature);
            witness.push(mulsig_script.clone());
            witness.push(&control_block.serialize());
        }
        SpendType::ESCAPE(_) => {
            witness.push(escape_script.clone());
            witness.push(&control_block.serialize());
        }
        _ => {}
    }
    unsigned_tx
        .input
        .iter_mut()
        .for_each(|v| v.witness = witness.clone());

    let fee = calculate_fee(unsigned_tx.vsize(), fee_rate, 1.0);
    println!("fee: {} sat", fee);

    if total_amount < amount + fee {
        panic!("invalid transaction amount, transfer out only: {}", total_amount - fee);
    }

    let owner_amount = total_amount - amount - fee;
    let dust_value = addr_self.script_pubkey().dust_value().to_sat();
    println!("owner_amount: {} sat", owner_amount);

    if owner_amount < dust_value {
        // remove second utxo
        unsigned_tx.output.pop();
    } else {
        // update the txOut's amount
        // the second utxo is for yourself.
        unsigned_tx.output[1].value = Amount::from_sat(owner_amount);
    }

    // build sign hash
    let inputs = unsigned_tx.input.len();
    let mut sighasher = SighashCache::new(&mut unsigned_tx);
    let prevouts: Vec<TxOut> = utxos
        .iter()
        .map(|v| TxOut {
            value: Amount::from_sat(v.amount),
            script_pubkey: addr_self.script_pubkey(),
        })
        .collect();

    let mut sig_hashs = Vec::<TapSighash>::new();
    for i in 0..inputs {
        let mock_script = ScriptBuf::new();
        let (is_script, script) = match spent_type {
            SpendType::ESCAPE(_) => (true, &escape_script),
            SpendType::MULSIG => (true, &mulsig_script),
            SpendType::KEY => (false, &mock_script),
        };
        let sig_hash: TapSighash = if is_script {
            sighasher
                .taproot_script_spend_signature_hash(
                    i,
                    &Prevouts::All(&prevouts),
                    TapLeafHash::from_script(script, LeafVersion::TapScript),
                    TapSighashType::Default,
                )
                .expect("failed to construct TapSighash")
        } else {
            sighasher
                .taproot_key_spend_signature_hash(
                    i,
                    &Prevouts::All(&prevouts),
                    TapSighashType::Default,
                )
                .expect("failed to construct TapSighash")
        };
        sig_hashs.push(sig_hash);
    }

    (unsigned_tx, sig_hashs, taproot_spend_info.merkle_root())
}
