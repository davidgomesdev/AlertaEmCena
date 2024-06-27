ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_DEB_ARCH

RUN dpkg --add-architecture $CROSS_DEB_ARCH
RUN apt-get update && apt-get install -y \
        libssl-dev:$CROSS_DEB_ARCH \
        pkg-config:$CROSS_DEB_ARCH
