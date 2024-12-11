#[cfg(test)]
mod tests {
    use bitcoin::key::{Keypair, TapTweak};
    use bitcoin::secp256k1::{rand, Secp256k1};
    use bitcoin::secp256k1::{PublicKey, XOnlyPublicKey};
    use bitcoin::{Address, Network};
    use std::str::FromStr;

    use secp256k1::curve::Scalar;

    use crate::{aux::*, LocalUtxo};
    use crate::builder::*;
    use crate::script::*;

    pub fn create_account(
        committee_secret: Vec<u8>,
        project_party_secret: Vec<u8>,
    ) -> (XOnlyPublicKey, XOnlyPublicKey, XOnlyPublicKey, Keypair) {
        let secp = Secp256k1::new();
        let committee_keypair = Keypair::from_seckey_slice(&secp, &committee_secret).unwrap();
        let committee_xonly = XOnlyPublicKey::from_keypair(&committee_keypair);

        let project_party_keypair =
            Keypair::from_seckey_slice(&secp, &project_party_secret).unwrap();
        let project_party_xonly = XOnlyPublicKey::from_keypair(&project_party_keypair);

        // add secrets
        let array_bytes: Result<[u8; 32], _> = committee_secret.as_slice().try_into();
        let mut scalar1 = Scalar::default();
        let _ = scalar1.set_b32(&array_bytes.unwrap());
        let array_bytes: Result<[u8; 32], _> = project_party_secret.as_slice().try_into();
        let mut scalar2 = Scalar::default();
        let _ = scalar2.set_b32(&array_bytes.unwrap());
        let scalar3 = scalar1 + scalar2;

        let sum_keypair = Keypair::from_seckey_slice(&secp, &scalar3.b32()).unwrap();
        let sum_xonly = XOnlyPublicKey::from_keypair(&sum_keypair);

        // add publics
        let combined_pk = committee_keypair
            .public_key()
            .combine(&project_party_keypair.public_key())
            .unwrap();
        let combined_xonly = combined_pk.x_only_public_key();
        assert_eq!(combined_xonly.0, sum_xonly.0);
        assert_eq!(combined_xonly.1, sum_xonly.1);

        // assert
        (
            committee_xonly.0,
            project_party_xonly.0,
            sum_xonly.0,
            sum_keypair,
        )
    }

    #[test]
    fn test_generate_key() {
        let secp = Secp256k1::new();
        let mut rng = rand::thread_rng();
        let (sk, pk) = secp.generate_keypair(&mut rng);
        println!("key pair: {:?}", hex::encode(sk.secret_bytes()));
        println!("key pair pk only: {:?}", hex::encode(pk.serialize()));
    }

    #[test]
    fn test_spent_with_mulsig_script() {
        let secp = Secp256k1::new();
        let pk = PublicKey::from_slice(&hex::decode("0480a4b120ad7f8ab294aa026952e7a359ae805e28afc13ad55fb44804bcfe9fe974e247aca11eafea1ab2501e396b39ff2b867c87e0e3b6418904b873b9909f3c").unwrap());
        println!("pk: {:?}", pk);
        let keypair1 = Keypair::new(&secp, &mut rand::thread_rng());
        let keypair2 = Keypair::new(&secp, &mut rand::thread_rng());
        println!(
            "keypair2 public key: {}",
            hex::encode(keypair2.public_key().serialize_uncompressed())
        );
        println!(
            "keypair2 public key: {}",
            hex::encode(keypair2.public_key().serialize())
        );

        let s1 = keypair1.secret_bytes().to_vec();
        let s2 = keypair2.secret_bytes().to_vec();
        let (a1, a2, sum, _) = create_account(s1.clone(), s2.clone());

        let mulsig2_script_builder = build_mulsig2(a1, a2);
        let escape_script_builder = build_escape(110, a2);

        let mulsig2_addr = build_p2tr_tweaked(
            sum,
            mulsig2_script_builder.clone().into_script(),
            escape_script_builder.clone().into_script(),
            Network::Regtest,
        );
        println!("mulsig2 addr: {}", mulsig2_addr);

        let out_points: Vec<LocalUtxo> = vec![LocalUtxo {
            txid: "0b78bd9e57b99e83bb1b5f1a1c1ecd8ae801fcb62f41cb62f986b38090354b65".to_string(),
            vout: 1,
            amount: 100000000,
        }];
        let receiver =
            Address::from_str("bcrt1pz7y5ps533cnjg8vhgjct6zt4zta8pc9tym6j39v52c37rj8tce7qzrzxj5")
                .unwrap()
                .assume_checked();
        let receiver_amount = 10000000;
        let fee_rate = 0.00001;

        let (tx, sighashs, _) = build_tx(
            SpendType::MULSIG,
            sum,
            mulsig2_script_builder.into_script(),
            escape_script_builder.into_script(),
            Network::Regtest,
            out_points,
            fee_rate,
            receiver,
            receiver_amount,
        );

        // sign by two users
        let sig1 = sign_transaction(sighashs.clone(), &s1);
        let sig2 = sign_transaction(sighashs, &s2);

        // combine the transaction.
        let tx_hex = combine_multi_sign_transaction(tx, sig1, sig2);

        // broadcast to bitcoin
        println!("{}", tx_hex);
    }

    #[test]
    fn test_spent_with_escape_script() {
        let secp = Secp256k1::new();
        let keypair1 = Keypair::new(&secp, &mut rand::thread_rng());
        let keypair2 = Keypair::new(&secp, &mut rand::thread_rng());
        let s1 = keypair1.secret_bytes().to_vec();
        let s2 = keypair2.secret_bytes().to_vec();
        let (a1, a2, sum, _) = create_account(s1.clone(), s2.clone());

        let lock_block = 110;
        let mulsig2_script_builder = build_mulsig2(a1, a2);
        let escape_script_builder = build_escape(lock_block, a2);

        let mulsig2_addr = build_p2tr_tweaked(
            sum,
            mulsig2_script_builder.clone().into_script(),
            escape_script_builder.clone().into_script(),
            Network::Regtest,
        );
        println!("mulsig2 addr: {}", mulsig2_addr);

        let out_points: Vec<LocalUtxo> = vec![LocalUtxo {
            txid: "b88cd14973cab9cd59d7e0e4f9fb36425ec671583fefce8c8d1341a848589ebe".to_string(),
            vout: 1,
            amount: 89999538,
        }];
        let receiver =
            Address::from_str("bcrt1pz7y5ps533cnjg8vhgjct6zt4zta8pc9tym6j39v52c37rj8tce7qzrzxj5")
                .unwrap()
                .assume_checked();
        let receiver_amount = 10000000;
        let fee_rate = 0.00005;

        let (tx, sighashs, _) = build_tx(
            SpendType::ESCAPE(lock_block as u32),
            sum,
            mulsig2_script_builder.into_script(),
            escape_script_builder.into_script(),
            Network::Regtest,
            out_points,
            fee_rate,
            receiver,
            receiver_amount,
        );

        // sign by project party
        let sig1 = sign_transaction(sighashs.clone(), &s2);

        // combine the transaction.
        let tx_hex = combine_escape_transaction(tx, sig1);

        // broadcast to bitcoin
        println!("{}", tx_hex);
    }

    #[test]
    fn test_spent_with_key() {
        let secp = Secp256k1::new();
        let keypair1 = Keypair::new(&secp, &mut rand::thread_rng());
        let keypair2 = Keypair::new(&secp, &mut rand::thread_rng());
        let s1 = keypair1.secret_bytes().to_vec();
        let s2 = keypair2.secret_bytes().to_vec();
        let (a1, a2, sum, sum_pair) = create_account(s1.clone(), s2.clone());

        let mulsig2_script_builder = build_mulsig2(a1, a2);
        let escape_script_builder = build_escape(110, a2);

        let mulsig2_addr = build_p2tr_tweaked(
            sum,
            mulsig2_script_builder.clone().into_script(),
            escape_script_builder.clone().into_script(),
            Network::Regtest,
        );
        println!("mulsig2 addr: {}", mulsig2_addr);

        let out_points: Vec<LocalUtxo> = vec![LocalUtxo {
            txid: "0b78bd9e57b99e83bb1b5f1a1c1ecd8ae801fcb62f41cb62f986b38090354b65".to_string(),
            vout: 1,
            amount: 100000000,
        }];
        let receiver =
            Address::from_str("bcrt1pz7y5ps533cnjg8vhgjct6zt4zta8pc9tym6j39v52c37rj8tce7qzrzxj5")
                .unwrap()
                .assume_checked();
        let receiver_amount = 10000000u64;
        let fee_rate = 0.00003;

        let (tx, sighashs, tweaked_hash) = build_tx(
            SpendType::KEY,
            sum,
            mulsig2_script_builder.into_script(),
            escape_script_builder.into_script(),
            Network::Regtest,
            out_points,
            fee_rate,
            receiver,
            receiver_amount,
        );

        // sign by tweaked key
        let tweaked_keypair: Keypair = sum_pair.tap_tweak(&secp, tweaked_hash).into();
        let sig1 = sign_transaction(sighashs.clone(), &tweaked_keypair.secret_bytes());

        // combine the transaction.
        let tx_hex = combine_key_transaction(tx, sig1);

        // broadcast to bitcoin
        println!("{}", tx_hex);
    }
}
