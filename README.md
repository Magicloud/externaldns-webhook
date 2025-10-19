# externaldns

[![crates](https://img.shields.io/crates/v/externaldns-webhook)](https://crates.io/crates/externaldns-webhook)
[![docs](https://img.shields.io/docsrs/externaldns-webhook/latest)](https://docs.rs/https://img.shields.io/docsrs/externaldns-webhook/latest)

This lib implenments External-DNS webhook interface. External-DNS uses this interface to communicate with non-built-in DNS service provider.

The main trait is `Provider`, which defines the four functions of the webhook. Pass the implementor to `Webhook::new`, then `Webhook::start` to get the whole thing running.

Ref: https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/webhook-provider.md