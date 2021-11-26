## 镜像构建
FROM ubuntu:20.04

RUN apt-get update
RUN apt-get install -y openssl

WORKDIR /opt/ttchain/

COPY ./docker/run.sh /opt/run.sh
COPY ./target/release/node-template /opt/ttchain/node-template

CMD /opt/run.sh
