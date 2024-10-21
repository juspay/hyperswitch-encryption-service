# Cripta

## Overview

A lightweight performant service to Encrypt and Decrypt your data and manage your Encryption Keys in a secure Storage.

The encryption service mainly has following functionalities:-
- **Encryption and Decryption**: Encrypt and decrypt data using secure algorithms and the managed DEKs.
- **Key Management**: Generate and Store Keys per entity which will be encrypted by a master key and stored in a secured manner.
- **Key Rotation**: Rotate DEKs on-demand to enhance security and comply with organizational policies.

## How does it work

- Application communicates with the service to create a key for the specific entity.
- Next time application has to encrypt/decrypt the data related to the entity, it has to send the entity identifier and the base64-encoded data, the Key Manager will encrypt/decrypt it for the application.
- All the communication between application and the encryption service are authorised by Mutual TLS
- All the Data Encryption Keys are Encrypted by either by securely generated AES-256 Key or a hosted Key Management Service (AWS KMS, Hashicorp Vault etc.)

![Architectural diagram](./docs/images/FlowDiagram.png)


## Development
- Cripta supports AES, AWS KMS, Hashicorp Vault as backends. Run cripta service with either of the backend by mentioning in feature flag. 
- `cargo run` chooses AES as the default backend. For other backends disable default-features flag and then choose the backend. Ex: `cargo run --no-default-features --features=vault` disables AES and chooses Hashicorp vault.
- Run `docker compose --file docker/development/docker-compose.yml --profile aes up -d` to setup all the required minimal services with AES as backend.

### Hashicorp Vault Setup
- Stop running services under a given backend. Ex: `docker compose --file docker/development/docker-compose.yml --profile aes down` will stop services based on AES backend.
- Run `docker compose --file docker/development/docker-compose.yml --profile vault up -d` to setup all the required services with vault as backend.
- Interact Hashicorp vault in the browser by accessing `http://localhost:8200/ui/vault/secrets`

#### FAQS
- If application complains for not finding `vault_token` make sure that `CRIPTA__secrets__vault_token` env variable is present with the vault access token.
