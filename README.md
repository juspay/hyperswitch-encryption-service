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
- Run `docker compose --file docker-compose.yml up -d` to run required services (postgres, Hashicorp vault) for development.