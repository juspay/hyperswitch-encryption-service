ALT="DNS:localhost"
CA_SUBJECT="/C=US/ST=CA/O=Cripta CA/CN=Cripta CA"
SUBJECT="/C=US/ST=CA/O=Cripta/CN=localhost"

function gen_ca() {
  openssl genrsa -out ca_key.pem 4096
  openssl req -new -x509 -days 3650 -key ca_key.pem \
    -subj "${CA_SUBJECT}" -out ca_cert.pem
}

function gen_ca_if_non_existent() {
  if ! [ -f ./ca_cert.pem ]; then gen_ca; fi
}

function gen_rsa_sha256() {
  gen_ca_if_non_existent

  openssl req -newkey rsa:4096 -nodes -sha256 -keyout rsa_sha256_key.pem \
    -subj "${SUBJECT}" -out server.csr

  openssl x509 -req -sha256 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
    -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
    -in server.csr -out rsa_sha256_cert.pem

  openssl pkcs12 -export -password pass:rocket \
    -in rsa_sha256_cert.pem -inkey rsa_sha256_key.pem -out rsa_sha256.p12

  rm ca_cert.srl server.csr
}

gen_rsa_sha256

