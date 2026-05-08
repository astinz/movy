use std::collections::{BTreeMap, BTreeSet};

use movy_fuzz::{
    input::MoveFuzzInput,
    meta::{FuzzMetadata, Metadata, TargetFilters},
    state::FuzzObjectStore,
    utils::SuperRand,
};
use movy_types::{
    input::{MoveAddress, MoveTypeTag},
    object::{MoveDigest, MoveObjectInfo, MoveOwner},
};

#[derive(Default)]
struct EmptyObjectStore;

impl FuzzObjectStore for EmptyObjectStore {
    fn get_move_object_info(
        &self,
        object: MoveAddress,
    ) -> Result<MoveObjectInfo, movy_types::error::MovyError> {
        Ok(MoveObjectInfo {
            id: object,
            ty: MoveTypeTag::Struct("0x2::coin::Coin".parse()?),
            owner: MoveOwner::Immutable,
            version: 1,
            digest: MoveDigest::new([0; 32]),
        })
    }
}

#[test]
fn core_metadata_and_input_compile_without_sui_executor() {
    let types_pool: BTreeMap<MoveTypeTag, BTreeSet<MoveAddress>> = BTreeMap::new();
    let base = Metadata::from_parts(BTreeMap::new(), BTreeMap::new(), types_pool, None, None);

    let meta = FuzzMetadata::from_metadata(
        base,
        SuperRand::new(1),
        vec![],
        vec![],
        MoveAddress::zero(),
        MoveAddress::zero(),
        MoveAddress::zero(),
        BTreeMap::new(),
        0,
        0,
        0,
        TargetFilters::default(),
    );
    let input = MoveFuzzInput::new();
    let store = EmptyObjectStore;

    assert!(meta.target_functions.is_empty());
    assert!(input.sequence.commands.is_empty());
    assert_eq!(
        store
            .get_move_object_info(MoveAddress::zero())
            .expect("core object store compiles")
            .id,
        MoveAddress::zero()
    );
}
