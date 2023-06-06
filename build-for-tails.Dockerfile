FROM rust:slim as bc-base
RUN apt update -y
RUN apt install -y libssl1.1 libssl-dev dpkg-dev build-essential wget tar gzip
WORKDIR /app
RUN wget https://www.openssl.org/source/openssl-1.1.1u.tar.gz 
RUN tar -zxvf openssl-1.1.1u.tar.gz 
WORKDIR openssl-1.1.1u
RUN ./config && make

FROM bc-base
WORKDIR /app
COPY . source
COPY .git source/.git
WORKDIR source
ENV OPENSSL_DIR=/app/openssl-1.1.1u
ENV OPENSSL_INCLUDE_DIR=/app/openssl-1.1.1u/include
ENV X86_64_UNKNOWN_LINUX_GNU_OPENSSL_LIB_DIR=/app/openssl-1.1.1u
RUN cargo install cargo-deb
RUN cargo deb