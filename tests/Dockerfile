FROM rust:1.39.0

RUN apt-get update && \
    apt-get install \
      apt-transport-https \
      ca-certificates \
      curl \
      gnupg2 \
      software-properties-common -y && \
    curl -fsSL https://download.docker.com/linux/debian/gpg | apt-key add  && \
    add-apt-repository \
      "deb [arch=amd64] https://download.docker.com/linux/debian \
      $(lsb_release -cs) \
      stable" && \
    apt-get update && \
    apt-get install docker-ce-cli -y && \
    rm -rf /var/lib/apt/lists/*
