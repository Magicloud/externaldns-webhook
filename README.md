# externaldns

[![crates](https://img.shields.io/crates/v/externaldns-webhook)](https://crates.io/crates/externaldns-webhook)
[![docs](https://img.shields.io/docsrs/externaldns-webhook/latest)](https://docs.rs/https://img.shields.io/docsrs/externaldns-webhook/latest)

This lib implenments External-DNS webhook interface. External-DNS uses this interface to communicate with non-built-in DNS service provider.

**The first thing to care is:**

```rust
trait Provider {
    async fn domain_filter(&self) -> Result<DomainFilter>;
    async fn records(&self) -> Result<Vec<Endpoint>>;
    async fn apply_changes(&self, changes: Changes) -> Result<()>;
    async fn adjust_endpoints(&self, endpoints: Vec<Endpoint>) -> Result<Vec<Endpoint>> {
        Ok(endpoints)
    }
}
```

The implementor must implement the first three functioons.

`domain_filter` tells External-DNS the rules to match the domains that this provider takes care.

`records` tells External-DNS all records the provider currently solves.

External-DNS tells `apply_changes` what (records) to CUD.

With this implementor, and an optional `Status` implementor, one can `Webhook::new()` to get a `Webhook` instance, then `Webhook::start()` to get everything working.

**For more reference, please checkout the example, which is a fully functioned provider for DNSMasq, which I am using in my K3S.**

Ref: [webhook-provider.md](https://github.com/kubernetes-sigs/external-dns/blob/master/docs/tutorials/webhook-provider.md)
