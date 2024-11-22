#!/bin/sh

set -eux -o pipefail

while true;
do
    dnsmasq "$@"
    inotifywait -r -e modify -e create /etc/dnsmasq.d/ &&
    killall dnsmasq
done