# externaldns

This lib implenments External-DNS webhook interface. External-DNS uses this interface to communicate with non-built-in DNS service provider.

The main trait is `Provider`, which defines the four functions of the webhook. Pass the implementor to `Webhook::new`, then `Webhook::start` to get the whole thing running.

Ref: https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/webhook-provider.md