FROM debian:latest

RUN echo "debian" > /etc/hostname

ENV LANG=en_US.UTF-8
ENV BLUEPRINT=debian
ENV CAPSULE_USERNAME=$CAPSULE_USERNAME

RUN apt-get update && apt-get install -y --no-install-recommends \
    sudo \
    curl \
    alsa-utils \
    pulseaudio \
    tmux \
    vim \
    bash

RUN apt-get install --reinstall -y ca-certificates

RUN echo "$CAPSULE_USERNAME ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
