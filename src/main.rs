#![allow(dead_code)]

mod aux;
mod builder;
mod command;
mod script;
mod test;

use aux::mulsig_address;
use bitcoin::Network;
use clap::Parser;
use command::*;
use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalUtxo {
    pub txid: String,
    pub vout: u32,
    pub amount: u64,
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    secret: String,

    #[clap(short, long, value_parser)]
    committee: String,

    #[clap(short, long, value_parser)]
    multisign: Option<String>,

    #[clap(short, long, value_parser)]
    time: u64,

    /// 0.00001 represents 1 sat/vB
    #[clap(long, value_parser, default_value = "0.00001")]
    fee_rate: f64,

    /// utxos list in json format.
    /// example: '[{"txid":"2946d93547be832d3fd63086c3894948a0f13ed29077d00aa5a3c8767ea83497","vout":0,"amount":10000000}]'
    #[clap(long, value_parser)]
    utxos: String,

    #[clap(short, long, value_parser)]
    receiver: String,

    #[clap(long, value_parser)]
    receiver_amount: u64,

    #[clap(short, long, value_parser)]
    network: u64,
}

fn main() {
    let args = Args::parse();

    println!("========= parameters =========");
    let private_key_u8 = hex::decode(args.secret).unwrap();
    let multi_signer = mulsig_address(args.multisign, &private_key_u8);
    let utxos: Vec<LocalUtxo> = serde_json::from_str(&args.utxos).unwrap();
    println!("multi_signer pk {}", multi_signer);
    println!("commitee pk {}", args.committee);
    println!("unlock time {}", args.time);
    println!("receive amount {} fee_rate {}", args.receiver_amount, args.fee_rate);

    let network = match args.network {
        0 => Network::Bitcoin,
        1 => Network::Testnet,
        2 => Network::Regtest,
        _ => Network::Testnet,
    };
    println!("network {}", network);
    println!("========= ========== =========");

    let tx = build_mulsig_escape_command(
        &private_key_u8,
        args.time,
        args.committee,
        multi_signer,
        args.fee_rate,
        args.receiver,
        args.receiver_amount,
        network,
        utxos
    );
    println!(">> tx: {}", tx);
}