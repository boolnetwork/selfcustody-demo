use bitcoin::key::Keypair;
use bitcoin::secp256k1::PublicKey;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::{Address, Network};
use std::str::FromStr;

use crate::aux::{combine_escape_transaction, sign_transaction};
use crate::builder::{build_tx, SpendType};
use crate::script::{build_escape, build_mulsig2};
use crate::LocalUtxo;

pub fn build_mulsig_escape_command(
    secret: &[u8],
    time: u64,
    committee: String,
    multi_signer: String,
    fee_rate: f64,
    receiver: String,
    receiver_amount: u64,
    network: Network,
    utxos: Vec<LocalUtxo>
) -> String {
    let secp = Secp256k1::new();
    let keypair = Keypair::from_seckey_slice(&secp, secret).unwrap();
    let committee = PublicKey::from_str(&committee).unwrap();
    let mulsigner = PublicKey::from_str(&multi_signer).unwrap();
    let sum = committee.combine(&mulsigner).unwrap();

    let mulsig2_script_builder = build_mulsig2(
        committee.x_only_public_key().0,
        mulsigner.x_only_public_key().0,
    );
    let escape_script_builder = build_escape(time, keypair.x_only_public_key().0);
    let receiver = Address::from_str(&receiver).unwrap().assume_checked();

    let (tx, sighashs, _) = build_tx(
        SpendType::ESCAPE(time as u32),
        sum.x_only_public_key().0,
        mulsig2_script_builder.into_script(),
        escape_script_builder.into_script(),
        network,
        utxos,
        fee_rate,
        receiver,
        receiver_amount,
    );

    // sign by project party
    let sig1 = sign_transaction(sighashs.clone(), secret);

    // combine the transaction.

    combine_escape_transaction(tx, sig1)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bitcoin::{
        key::{Secp256k1, TweakedPublicKey},
        Address, Network, XOnlyPublicKey,
    };

    #[test]
    fn pk2tr() {
        //Test case from BIP-086
        let internal_key = XOnlyPublicKey::from_str(
            "796b3c4d46db59b4958080dc426168283901d70d7582cf8d43a29c30d3b84441",
        )
        .unwrap();
        let secp = Secp256k1::verification_only();
        let address = Address::p2tr(&secp, internal_key, None, Network::Testnet);
        println!("address: {}", address);

        let tweaked_key = XOnlyPublicKey::from_str(
            "796b3c4d46db59b4958080dc426168283901d70d7582cf8d43a29c30d3b84441",
        )
        .unwrap();
        let address = Address::p2tr_tweaked(
            TweakedPublicKey::dangerous_assume_tweaked(tweaked_key),
            Network::Testnet,
        );
        println!("address: {}", address);
    }
}
