-- Do not run this migration in production environments.
-- Dropping this column is a backward-incompatible change: the data will be
-- permanently lost, and any rollback to a version that still relies on this
-- column will break. Only run this once backward compatibility with older
-- application versions is no longer required.
ALTER TABLE data_key_store DROP COLUMN IF EXISTS token;
