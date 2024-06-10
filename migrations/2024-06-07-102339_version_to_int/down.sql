ALTER TABLE data_key_store ALTER COLUMN version TYPE VARCHAR(30) using (version::varchar);
