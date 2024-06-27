ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

ARG CROSS_DEB_ARCH

RUN dpkg --add-architecture $CROSS_DEB_ARCH
RUN apt-get update && apt-get -y install libudev-dev:$CROSS_DEB_ARCH libsystemd-dev:$CROSS_DEB_ARCH libusb-1.0-0-dev:$CROSS_DEB_ARCH libusb-0.1-4:$CROSS_DEB_ARCH
RUN apt-get update && apt-get install -y \
        openssl:$CROSS_DEB_ARCH \
        libssl-dev:$CROSS_DEB_ARCH \
        pkg-config:$CROSS_DEB_ARCH
