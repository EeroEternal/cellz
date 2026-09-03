use cellz::storage::{BlobStore, S3BlobStore};

#[tokio::test]
async fn test_cloudflare_r2_live_roundtrip() {
    let (Ok(endpoint), Ok(bucket), Ok(access_key), Ok(secret_key)) = (
        std::env::var("CELLZ_S3_ENDPOINT"),
        std::env::var("CELLZ_S3_BUCKET"),
        std::env::var("CELLZ_S3_ACCESS_KEY_ID"),
        std::env::var("CELLZ_S3_SECRET_ACCESS_KEY"),
    ) else {
        eprintln!("Skipping Cloudflare R2 test: CELLZ_S3_* credentials not set in environment");
        return;
    };

    let store = S3BlobStore::new(&endpoint, &bucket, &access_key, &secret_key, Some("auto"))
        .expect("Failed to initialize S3BlobStore for Cloudflare R2");

    let test_path = format!("test-probe-{}.txt", uuid::Uuid::new_v4());
    let test_payload = b"Hello Cloudflare R2 from cellz!".to_vec();

    // 1. Put object
    store
        .put(&test_path, test_payload.clone())
        .await
        .expect("Failed to put test object into Cloudflare R2");

    // 2. Exists
    let exists = store
        .exists(&test_path)
        .await
        .expect("Failed to check object existence");
    assert!(exists, "Object must exist in R2");

    // 3. Get object
    let fetched = store
        .get(&test_path)
        .await
        .expect("Failed to get object from Cloudflare R2");
    assert_eq!(fetched, test_payload);

    // 4. Test lease acquire
    let lease_key = format!("test-lease-{}", uuid::Uuid::new_v4());
    let acquired = store
        .acquire_lease(&lease_key, "worker-node-1", 30)
        .await
        .expect("Failed to acquire lease in R2");
    assert!(acquired, "Should acquire fresh lease");

    // 5. Competing node should be denied
    let competing = store
        .acquire_lease(&lease_key, "worker-node-2", 30)
        .await
        .expect("Failed to check competing lease");
    assert!(!competing, "Competing node must not acquire active lease");

    // 6. Same node renewal
    let renewed = store
        .renew_lease(&lease_key, "worker-node-1", 60)
        .await
        .expect("Failed to renew lease");
    assert!(renewed, "Original holder must be able to renew lease");

    // 7. Cleanup
    store.release_lease(&lease_key, "worker-node-1").await.ok();
    store.delete(&test_path).await.ok();

    println!("✅ Cloudflare R2 live verification passed successfully!");
}
