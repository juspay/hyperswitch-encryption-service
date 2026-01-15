# DEK Re-encryption API

## Overview

The `/key/reencrypt` endpoint allows you to re-encrypt existing Data Encryption Keys (DEKs) with a new KMS key **without changing their version numbers**. This is useful for migrating from one KMS key to another (e.g., single-region to multi-region KMS keys).

## Important Prerequisites

**REQUIRED**: `skip_key_id_on_decrypt` must be set to `true` in your KMS configuration. This allows AWS KMS to determine which key to use for decryption from the ciphertext metadata.

## API Endpoint

**POST** `/key/reencrypt`

## Request Body

### Re-encrypt Specific Identifier
```json
{
  "data_identifier": "User",
  "key_identifier": "123"
}
```

### Re-encrypt All DEKs (Use with caution!)
```json
{}
```

### Parameters

- `data_identifier` (optional): The type of data
- `key_identifier` (optional): The unique identifier for the key
- If both are omitted, **ALL DEKs** in the database will be re-encrypted

## Response

```json
{
  "total_keys": 10,
  "success_count": 9,
  "failure_count": 1
}
```

### Response Fields

- `total_keys`: Total number of DEKs attempted
- `success_count`: Number of successfully re-encrypted DEKs
- `failure_count`: Number of DEKs that failed to re-encrypt

## How It Works

1. **Fetch DEKs**: Retrieves all DEKs matching the criteria (specific identifier or all)
2. **Decrypt**: Decrypts each DEK using the old KMS key (determined automatically from ciphertext)
3. **Re-encrypt**: Encrypts the DEK using the new KMS key (from current config)
4. **Update**: Stores the re-encrypted DEK back in the database **with the same version**

## Use Cases

### 1. Migrate Single-Region to Multi-Region KMS Key

```bash
# Step 1: Update config with skip_key_id_on_decrypt = true and new KMS key
# Step 2: Redeploy service
# Step 3: Re-encrypt specific identifiers

curl -X POST http://localhost:8080/key/reencrypt \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID: tenant1" \
  -d '{
    "data_identifier": "User",
    "key_identifier": "123"
  }'
```

### 2. Bulk Re-encryption

```bash
# WARNING: This re-encrypts ALL DEKs - use carefully!
curl -X POST http://localhost:8080/key/reencrypt \
  -H "Content-Type: application/json" \
  -H "X-Tenant-ID": "tenant1" \
  -d '{}'
```

## Important Differences from `/key/rotate`

| Feature | `/key/rotate` | `/key/reencrypt` |
|---------|--------------|------------------|
| Creates new version | ✅ Yes (v1 → v2) | ❌ No (stays v1) |
| Changes DEK value | ✅ New random DEK | ❌ Same DEK |
| Can specify new key | ❌ No | ✅ Yes (via config) |
| Use case | Regular key rotation | KMS key migration |
| Requires `skip_key_id_on_decrypt` | ❌ No | ✅ Yes |

## Configuration Example

```toml
[secrets.kms_config]
key_id = "arn:aws:kms:us-east-1:123456789012:key/mrk-new-key"
region = "us-east-1"
skip_key_id_on_decrypt = true  # REQUIRED for re-encryption
```

## Migration Strategy

### Safe Migration Path:

1. **Backup**: Backup your database before starting
2. **Enable Flag**: Set `skip_key_id_on_decrypt = true` in config
3. **Update Key**: Change `key_id` to your new KMS key
4. **Deploy**: Redeploy the service
5. **Test**: Re-encrypt a single identifier first
   ```bash
   POST /key/reencrypt
   {"data_identifier": "Test", "key_identifier": "1"}
   ```
6. **Verify**: Ensure encryption/decryption still works
7. **Migrate**: Re-encrypt remaining identifiers one by one or in batches
8. **Monitor**: Check logs for any failures

### Monitoring

The endpoint logs each DEK re-encryption with:
- ✅ Success: `Successfully re-encrypted DEK`
- ❌ Failure: `Failed to re-encrypt DEK` with error details

## Error Handling

### Common Errors

**400 Bad Request**
- Invalid request format

**500 Internal Server Error**
- `skip_key_id_on_decrypt must be enabled`: The flag is not enabled in config
- `KMS operation failed`: AWS KMS permissions or key issues
- `Database error`: Failed to update DEK in database

## Limitations

### Cassandra Users
- `get_all_keys()` (re-encrypting all DEKs without identifier) is **not supported**
- You must specify `data_identifier` and `key_identifier`
- This is due to Cassandra's architecture not supporting efficient full table scans

### Performance Considerations
- Re-encrypting many DEKs can take time
- Each DEK requires 2 KMS calls (decrypt + encrypt)
- Consider rate limiting if you have thousands of DEKs
- Run during low-traffic periods for bulk operations

## Security Notes

1. **IAM Permissions**: Your service must have permissions for:
   - Old KMS key: `kms:Decrypt`
   - New KMS key: `kms:Encrypt`, `kms:GenerateDataKey`

2. **Audit Trail**: All re-encryption operations are logged

3. **No Downtime**: The service continues to work during re-encryption
   - Old DEKs: Still decrypt using old key
   - New operations: Use new key
   - Re-encrypted DEKs: Now use new key

4. **Rollback**: If needed, you can re-encrypt back to the old key by:
   - Reverting the `key_id` in config
   - Redeploying
   - Running re-encryption again
