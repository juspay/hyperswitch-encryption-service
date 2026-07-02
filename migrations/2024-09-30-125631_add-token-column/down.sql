-- This file should undo anything in `up.sql`
ALTER TABLE data_key_store DROP COLUMN IF EXISTS token;
