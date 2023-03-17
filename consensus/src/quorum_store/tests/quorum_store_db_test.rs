// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_store::PersistRequest,
    quorum_store_db::{QuorumStoreDB, QuorumStoreStorage},
    tests::utils::create_vec_signed_transactions,
    types::Batch,
};
use aptos_consensus_types::proof_of_store::BatchId;
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress;

#[test]
fn test_db_for_data() {
    let tmp_dir = TempPath::new();
    let db = QuorumStoreDB::new(&tmp_dir);

    let source = AccountAddress::random();
    let signed_txns = create_vec_signed_transactions(100);
    let persist_request_1: PersistRequest =
        Batch::new(BatchId::new_for_test(1), signed_txns, 1, 20, source).into();
    let digest_1 = persist_request_1.digest;
    let value_1 = persist_request_1.value;
    assert!(db.save_batch(digest_1, value_1.clone()).is_ok());

    assert_eq!(
        db.get_batch(&digest_1)
            .expect("could not read from db")
            .unwrap(),
        value_1
    );

    let signed_txns = create_vec_signed_transactions(200);
    let persist_request_2: PersistRequest =
        Batch::new(BatchId::new_for_test(1), signed_txns, 1, 20, source).into();
    let digest_2 = persist_request_2.digest;
    let value_2 = persist_request_2.value;
    assert!(db.save_batch(digest_2, value_2).is_ok());

    let signed_txns = create_vec_signed_transactions(300);
    let persist_request_3: PersistRequest =
        Batch::new(BatchId::new_for_test(1), signed_txns, 1, 20, source).into();
    let digest_3 = persist_request_3.digest;
    let value_3 = persist_request_3.value;
    assert!(db.save_batch(digest_3, value_3).is_ok());

    let batches = vec![digest_3];
    assert!(db.delete_batches(batches).is_ok());
    assert_eq!(
        db.get_batch(&digest_3).expect("could not read from db"),
        None
    );

    let all_batches = db.get_all_batches().expect("could not read from db");
    assert_eq!(all_batches.len(), 2);
    assert!(all_batches.contains_key(&digest_1));
    assert!(all_batches.contains_key(&digest_2));
}

#[test]
fn test_db_for_batch_id() {
    let tmp_dir = TempPath::new();
    let db = QuorumStoreDB::new(&tmp_dir);

    assert!(db
        .clean_and_get_batch_id(0)
        .expect("could not read from db")
        .is_none());
    assert!(db.save_batch_id(0, BatchId::new_for_test(0)).is_ok());
    assert!(db.save_batch_id(0, BatchId::new_for_test(4)).is_ok());
    assert_eq!(
        db.clean_and_get_batch_id(0)
            .expect("could not read from db")
            .unwrap(),
        BatchId::new_for_test(4)
    );
    assert!(db.save_batch_id(1, BatchId::new_for_test(1)).is_ok());
    assert!(db.save_batch_id(2, BatchId::new_for_test(2)).is_ok());
    assert_eq!(
        db.clean_and_get_batch_id(2)
            .expect("could not read from db")
            .unwrap(),
        BatchId::new_for_test(2)
    );
}
