#! /usr/bin/env bash

# Additional domains
DOMAINS=()

while [[ $# -gt 0 ]]; do
  case $1 in
    --prod)
      PROD="true"
      shift # past argument
      ;;
    --namespace)
      NAMESPACE="$2"
      shift # past argument
      shift # past value
      ;;
    --service)
      SERVICE="$2"
      shift # past argument
      shift # past value
      ;;
    --domain)
      DOMAINS+=("$2")
      shift # past argument
      shift # past value
      ;;
    --client-name)
      CLIENT_NAME="$2"
      shift # past argument
      shift # past value
      ;;
    --force-server)
      FORCE_SERVER="true"
      shift # past argument
      ;;
    -*|--*)
      echo "Unknown option $1"
      exit 1
      ;;
  esac
done

# Check required arguments for production mode
if [[ "$PROD" == "true" && (-z "$NAMESPACE" || -z "$SERVICE") ]]; then
    echo "Error: --namespace and --service are required when using --prod"
    exit 1
fi

# Check client name for invalid characters
if [[ -n "$CLIENT_NAME" && ! "$CLIENT_NAME" =~ ^[a-zA-Z0-9_-]+$ ]]; then
    echo "Error: Client name can only contain letters, numbers, hyphens, and underscores"
    exit 1
fi

if [[ "$PROD" == "true" ]]; then
    ALT=$(printf "DNS:%s.%s.svc.cluster.local" $SERVICE $NAMESPACE)
    CA_SUBJECT="/C=US/ST=CA/O=Cripta CA/CN=Cripta CA"
    SUBJECT=$(printf "/C=US/ST=CA/O=Cripta/CN=%s.%s.svc.cluster.local" $SERVICE $NAMESPACE)
else
    ALT="DNS:localhost"
    CA_SUBJECT="/C=US/ST=CA/O=Cripta CA/CN=Cripta CA"
    SUBJECT="/C=US/ST=CA/O=Cripta/CN=localhost"
fi

# Append additional domains to ALT
for domain in "${DOMAINS[@]}"; do
    ALT="${ALT},DNS:${domain}"
done

function gen_ca() {
  openssl genrsa -out ca_key.pem 2048
  openssl req -new -x509 -days 3650 -key ca_key.pem \
    -subj "${CA_SUBJECT}" -out ca_cert.pem
}

function gen_ca_if_non_existent() {
  if ! [ -f ./ca_cert.pem ]; then gen_ca; fi
}

function gen_client_cert_key_pair() {
    local client_name="${1:-client}"  # Default to "client" if no name provided

    if [[ -f "client_${client_name}.pem" ]]; then
        echo "Warning: Overwriting existing client certificate files for '${client_name}'"
    fi

    # Generate client private key and CSR
    openssl req -newkey rsa:2048 -nodes -sha256 -keyout "client_${client_name}_key.pem" \
        -subj "/C=US/ST=CA/O=Cripta Client/CN=${client_name}" \
        -out "client_${client_name}.csr"

    # Sign client certificate with CA
    openssl x509 -req -sha256 -days 3650 \
        -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
        -in "client_${client_name}.csr" -out "client_${client_name}_cert.pem"

    # Combine client cert and key
    cat "client_${client_name}_cert.pem" "client_${client_name}_key.pem" > "client_${client_name}.pem"

    # Clean up serial and CSR files
    rm "ca_cert.srl" "client_${client_name}.csr"
}

function gen_rsa_sha256() {
  gen_ca_if_non_existent

  openssl req -newkey rsa:2048 -nodes -sha256 -keyout rsa_sha256_key.pem \
    -subj "${SUBJECT}" -out server.csr

  openssl x509 -req -sha256 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
    -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
    -in server.csr -out rsa_sha256_cert.pem

  rm ca_cert.srl server.csr
}

# Generate server certificate and private key (only if they don't exist or if forced)
if [[ ! -f "rsa_sha256_cert.pem" || "$FORCE_SERVER" == "true" ]]; then
    if [[ -f "rsa_sha256_cert.pem" && "$FORCE_SERVER" == "true" ]]; then
        echo "Warning: Overwriting existing server certificate files"
    fi
    gen_rsa_sha256
else
    echo "Info: Server certificate files already exist, skipping generation (use --force-server to overwrite)"
fi

# Ensure CA exists for client cert signing
gen_ca_if_non_existent
# Generate client certificate with provided name (or default to "client")
gen_client_cert_key_pair "${CLIENT_NAME:-client}"

