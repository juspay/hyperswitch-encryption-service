#!/bin/bash

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
    -*|--*)
      echo "Unknown option $1"
      exit 1
      ;;
  esac
done

if [ "$PROD" == "true" ]; then
    ALT=$(printf "DNS:%s.%s.svc.cluster.local" $NAMESPACE $SERVICE)
    CA_SUBJECT="/C=US/ST=CA/O=Cripta CA/CN=Cripta CA"
    SUBJECT=$(printf "/C=US/ST=CA/O=Cripta/CN=%s.%s.svc.cluster.local" $NAMESPACE $SERVICE)
else
    ALT="DNS:localhost"
    CA_SUBJECT="/C=US/ST=CA/O=Cripta CA/CN=Cripta CA"
    SUBJECT="/C=US/ST=CA/O=Cripta/CN=localhost"
fi

function gen_ca() {
  openssl genrsa -out ca_key.pem 4096
  openssl req -new -x509 -days 3650 -key ca_key.pem \
    -subj "${CA_SUBJECT}" -out ca_cert.pem
}

function gen_ca_if_non_existent() {
  if ! [ -f ./ca_cert.pem ]; then gen_ca; fi
}

function gen_client_key() {
    cat rsa_sha256_cert.pem rsa_sha256_key.pem  > client.pem
}

function gen_rsa_sha256() {
  gen_ca_if_non_existent

  openssl req -newkey rsa:4096 -nodes -sha256 -keyout rsa_sha256_key.pem \
    -subj "${SUBJECT}" -out server.csr

  openssl x509 -req -sha256 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
    -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
    -in server.csr -out rsa_sha256_cert.pem

  rm ca_cert.srl server.csr
}

gen_rsa_sha256
gen_client_key

