FROM rust:1.43
MAINTAINER Simon Egersand "s.egersand@gmail.com"

RUN cargo install urlsup

VOLUME /mnt
WORKDIR /mnt

ENTRYPOINT ["urlsup"]
CMD ["--help"]
