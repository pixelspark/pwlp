#!/bin/sh
cargo build --target=arm-unknown-linux-musleabi --features=raspberrypi 
strip ./target/arm-unknown-linux-musleabi/release/pwlp 
scp ./target/arm-unknown-linux-musleabi/release/pwlp rpi:/home/pi/