-- Your SQL goes here
ALTER TABLE data_key_store ADD COLUMN IF NOT EXISTS token VARCHAR(255);
