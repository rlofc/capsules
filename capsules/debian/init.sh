#!/bin/bash
echo "$CAPSULE_USERNAME ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

apt-get update && apt-get install -y --no-install-recommends \
  # fonts-inter \
  # ..and any other package you want to install on top \
  # ..of the docker image 
