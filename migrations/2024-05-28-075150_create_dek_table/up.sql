CREATE TABLE IF NOT EXISTS data_key_store (
    id SERIAL PRIMARY KEY,
    key_identifier VARCHAR(255) NOT NULL,
    data_identifier VARCHAR(20) NOT NULL,
    encryption_key bytea NOT NULL,
    version VARCHAR(30) NOT NULL,
    created_at TIMESTAMP NOT NULL
);

CREATE UNIQUE INDEX data_key_identifier_unique_index ON data_key_store(key_identifier,data_identifier,version);
