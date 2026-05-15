FROM debian:latest

ENV LANG=en_US.UTF-8
ARG CAPSULE_USERNAME
ENV CAPSULE_USERNAME=$CAPSULE_USERNAME

RUN apt-get update && apt-get install -y --no-install-recommends \
    locales \
    sudo \
    curl \
    alsa-utils \
    pulseaudio \
    git \
    unzip \
    tmux \
    vim \
    bash \
    && sed -i '/en_US.UTF-8/s/^# //g' /etc/locale.gen \
    && dpkg-reconfigure --frontend=noninteractive locales \
    && update-locale LANG=en_US.UTF-8

RUN apt-get install --reinstall -y ca-certificates

RUN echo "$CAPSULE_USERNAME ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
