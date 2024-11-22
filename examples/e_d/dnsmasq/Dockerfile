FROM alpine:latest

VOLUME [ "/etc/dnsmasq.d/" ]

RUN apk add --no-cache dnsmasq inotify-tools
COPY dnsmasq.conf /etc/dnsmassq.conf
COPY reload.sh /reload.sh

EXPOSE 53/TCP
EXPOSE 53/UDP

CMD [ "/reload.sh" ]