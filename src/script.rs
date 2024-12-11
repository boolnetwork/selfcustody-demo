use bitcoin::opcodes::all::{OP_CHECKSIG, OP_CHECKSIGVERIFY, OP_CLTV, OP_DROP};
use bitcoin::script::Builder;
use bitcoin::secp256k1::XOnlyPublicKey;

pub fn build_mulsig2(committee_pk: XOnlyPublicKey, project_party_pk: XOnlyPublicKey) -> Builder {
    Builder::new()
        .push_x_only_key(&project_party_pk)
        .push_opcode(OP_CHECKSIGVERIFY)
        .push_x_only_key(&committee_pk)
        .push_opcode(OP_CHECKSIG)
}

pub fn build_escape(release_time: u64, project_party_pk: XOnlyPublicKey) -> Builder {
    Builder::new()
        .push_int(release_time as i64)
        .push_opcode(OP_CLTV)
        .push_opcode(OP_DROP)
        .push_x_only_key(&project_party_pk)
        .push_opcode(OP_CHECKSIG)
}
