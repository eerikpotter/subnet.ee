//! Catalog variants share the current adapter contract.
use candid::{decode_one, encode_one};
use sovereign_cloud::{
    ObjectId,
    drive::{FileFormat, ObjectKind},
};
#[test]
fn catalog_variants_round_trip_without_a_numeric_identity_contract() {
    let target = ObjectId::from_random_bytes([17; 16]);
    for kind in [ObjectKind::Folder, ObjectKind::File(FileFormat { media_type: "text/plain".into(), craft: None }), ObjectKind::Shortcut(target)] {
        assert_eq!(decode_one::<ObjectKind>(&encode_one(&kind).unwrap()).unwrap(), kind);
    }
}
